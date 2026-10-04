# Distribución de `wow-world` — programa #1241, continuación #1263

**Aceptación P4a — 2026-10-02:** F0–F3 y F4a P1–P3 integradas;
P4a aceptada en `d9c9e3637`, con publicación/integración registradas en #1263.
**Responsabilidad pendiente:** [#1263](https://github.com/alseif0x/rustycore/issues/1263),
continuación de [#1241](https://github.com/alseif0x/rustycore/issues/1241) bajo #584.
#1241 ya está cerrada en GitHub; ese estado no demuestra que F4–F6 estén terminadas.

**Checkpoint de pausa solicitado por el usuario — 2026-10-04.** Se detuvo a los
tres trabajadores para conservar el avance de #1263 y preparar commit/publicación.
El checkpoint es WIP y no acredita compilación, pruebas ni aceptación de F5/F6.
RawEquip conserva su cuerpo App, fachada y pruebas escritas, pero faltan montajes
y resolver el préstamo simultáneo de vitals mutables de Stats y Registry. Full
LootRelease tiene cuerpos de autoridad, publicación y ramas escritos, pero faltan
integración del consumidor, pruebas y la publicación Gathering dependiente de GO.
El proveedor readonly de activación/DynamicFlags de GO está diseñado, todavía sin
implementar; refresh completo y CompleteQuest siguen pendientes. Full Save,
completions de persistencia, compra Trainer y StorageMove/child/offhand/swap
completos conservan sus pendientes. No se cierra #1263 ni se autoriza merge/runtime.
El plan técnico general sigue siendo [refactor-completion-plan.md](refactor-completion-plan.md).
Este documento mantiene las decisiones, el estado fechado y los criterios de aceptación;
#1263 es su lista operativa de trabajo restante. Sustituye el enfoque anterior de #1233
(una rama, validación diferida). Las aceptaciones acotadas de P3/P4a se registran en §8;
no revalida las fases anteriores ni reclama nueva paridad o ahorro de build.

Base original del programa: `3.4.3` @ `e786ece1`; base de P4a: P3 integrada `1f8a7c800`. Una rama y un PR por fase (F0, F1, ...) o por dominio, integrados de
forma continua en `3.4.3`. La continuación conserva el alcance y las condiciones del programa
#1241; no amplía la autoridad de publicación, runtime o base de datos. La excepción de proceso
está en [AGENTS.md](../../AGENTS.md) ("#1241 wow-world split programme").

Referencia de capas, nombres, visibilidad y colocación de tests:
[structure-and-conventions.md](structure-and-conventions.md). Los presupuestos físicos y
excepciones los mantiene [module-design-guidelines.md](module-design-guidelines.md).
El orden global del workspace vive
en [workspace-structure-programme.md](workspace-structure-programme.md).

## 1. Diagnóstico (medido en `3.4.3` @ `e786ece1`)

| medida | valor |
|---|---:|
| líneas `.rs` de `wow-world/src` | ~412 000 |
| …tests/fixtures dentro de `src` (por ruta) | **224 620 (55 %)**; `session/tests/` solo: 124 918 |
| …producción | 187 252 |
| campos de `WorldSession` (`session/state.rs:398`) | **455** (306 con `cfg`) |
| métodos `impl WorldSession` | **3 834**, en 344 ficheros de producción |
| `crates/wow-world/tests/` (integración) | 3 016 |

La fuente de estas cifras y del mapa de acoplamiento es ahora
`python3 tools/architecture/wow_world_coupling.py report` (medido en `e786ece1`; incluye los
bloques `impl crate::session::WorldSession`/`super::` y excluye funciones anidadas); las cifras
anteriores de la issue (460 campos, 3 625 métodos, 323 ficheros) eran de un prototipo.

**Causa raíz:** los bloques `impl WorldSession` inherentes solo pueden vivir en el crate que define
`WorldSession`. El código que recibe `&mut WorldSession` no puede salir de `wow-world` por muchos
submódulos que se creen: partir ficheros dentro del crate no es distribuir.

**Por qué fallaron los intentos previos** (rama `584-map-manager-domain`, `e786ece1..5b443512`):

| crate | base | HEAD | Δ |
|---|---:|---:|---:|
| wow-world | 414 888 | 411 286 | **−3 602 (−0,9 %)** |
| wow-map | 53 516 | 113 049 | +59 533 |
| wow-entities | 79 797 | 102 228 | +22 431 |
| world-server | 60 438 | 72 312 | +11 874 |

1. Se construyeron rutas "canónicas" paralelas sin retirar las legadas: se copió en vez de mover.
2. Mover, rediseñar y documentar se hicieron en el mismo paso.
3. La validación se difirió al final: 231 commits WIP que no compilan; la puerta de publicación
   sigue roja (188 entradas obsoletas del baseline de campos de `WorldSession`, crecimiento del
   ratchet de hotspots, p. ej. +19 025 en el agregado `wow-map/src/map/mod.rs`, y una arista
   `wow-world -> wow-spell` sin dueño).

**Mapa de acoplamiento** (ficheros de producción, `3.4.3`):

- Campos hub (dominios que los usan; van a `SessionCore`): `account_id` (49),
  `player_handle_like_cpp` (47), `canonical_map_manager` (29), `lifecycle` (24),
  `client_visible_guids_like_cpp` (16), `state` (16), `spell_catalogs` (15), `player_registry`
  (14); 19 campos en total con ≥8 dominios. `player_guid` apenas se accede como campo (8 dominios):
  su uso es el accesor `self.player_guid()`, de `session/player_binding`.
- Dueños más llamados (llamadas entrantes, base del DAG): `session/player_items` 771,
  `session/player_binding` 740, `session/publication` 648, `session/canonical_access` 586,
  `session/instances` 548, `session/spell_state` 407, `session/world_entities` 245,
  `session/movement` 235, `session/quest` 228, `session/social` 211.
- Consumidores más pesados (cima): `handlers/character` 1 358, `handlers/loot` 606,
  `session/spell_effects` 498, `handlers/quest` 335, `handlers/group` 198.
- Aristas mayores: `character -> player_items` 397, `character -> publication` 196,
  `loot -> player_items` 132, `quest handlers -> session/quest` 118, `loot -> instances` 93,
  `group -> social` 74.

## 2. Decisiones aprobadas (usuario, 2026-09-30)

1. Partir de `3.4.3` limpio y aparcar `584-map-manager-domain`; sin rescate commit a commit.
2. Un PR por fase o dominio, integrado continuamente en `3.4.3`.
3. **Mover primero, rediseñar después.** Un paso solo-movimiento conserva sus anclas C++ y no
   requiere nueva re-auditoría C++ ni comparación por helper. Los cambios de comportamiento van a
   F6 con evidencia de paridad.
4. Nombres de crate `crates/wow-world-<dominio>`, en el layout plano, crate igual que su carpeta.
5. Decisiones de F4 conservadas de los comentarios de #1241: el hub y sus dependencias van a
   `wow-world-core`; el `map_manager` legado se mueve allí y su retirada pertenece a F6;
   los fixtures cruzan el límite mediante `test-fixtures`.
6. Visibilidad interna: en crates `wow-world-*` con `publish = false`, un miembro trasladado
   puede ser `pub` cuando un consumidor entre crates lo necesite. Revisar cada ampliación,
   limitar los reexports y conservar las invariantes; `publish = false` no aísla el estado.
   No se expone ese estado en `wow-module-api` ni se crean copias o escritores nuevos.
   P4a debe trasladar esta excepción acotada a AGENTS.md y a la guía de desarrollo.

## 3. Reglas de ejecución

- **R1 Mover, nunca copiar.** El mismo commit borra el origen. La comprobación net-move rechaza el
  PR si `wow-world` no encoge aproximadamente lo que crece el destino.
- **R2 Sin rediseño dentro de un movimiento.** Sin cambio semántico, sin renombres, sin nueva ruta
  canónica. Las anclas C++ viajan sin cambios.
- **R3 Cada paso compila.** Cada PR pasa `cargo check -p wow-world --all-targets` (más el crate del
  dominio movido) y los tests del dominio movido. Integración continua; sin rama WIP larga.
- **R4 Codemods, no reescritura a mano.** Ediciones masivas (`self.foo` -> `self.dom.foo`) con
  scripts guiados por el compilador.
- **R5 Una métrica pública:** líneas de producción de `wow-world` y número de métodos
  `impl WorldSession`. Sin porcentajes por helper ni párrafos de checkpoint.
- Los PRs parciales enlazan #1263 como seguimiento; no usan `Closes #1263` hasta
  satisfacer el cierre completo de F4–F6. La integración de una fase no cierra el programa.
- Siguen vigentes las restricciones congeladas de #1233: firmas visibles a handlers, registros,
  bytes y orden de paquetes, admisión por fase, persistencia/recuperación de COMMIT, locks y
  dueños de runtime.

## 4. Compatibilidad con el sistema de módulos (#583)

- **M1:** `wow-module-api` y `wow-script` siguen en la capa base; los crates de dominio pueden
  depender de ellos, nunca al revés. La superficie de módulos nunca expone `WorldSession` ni
  ningún `<Dominio>State`.
- **M2:** registro de módulos y handles de hooks son estado hub: viven en `SessionCore` y llegan a
  la lógica por su `<Dominio>Cx`. Los puntos de llamada de hooks se mueven **sin cambios** (mismo
  punto, orden y contexto).
- **M3:** cada `wow-world-<dominio>` es el dueño natural de los hooks de su dominio; añadir hooks
  nuevos es trabajo de #583, no de un paso de movimiento.
- **M4:** el `register()` por crate de F5 usa el mismo patrón de registro invertido que la
  composición de módulos; `world-server` / `world-modules` siguen siendo los únicos puntos de
  composición.
- **Aceptación:** cada paso F3/F4 mantiene verdes los tests de módulos/hooks y el fixture
  `compose.py check`.

## 5. Arquitectura objetivo

1. **`SessionCore` más un struct de estado por dominio:** `WorldSession` = `core: SessionCore`
   (identidad, conexión, handles de mapa/registro, catálogos) + `items: ItemsState`,
   `loot: LootState`, etc.
2. **La lógica nunca recibe `&mut WorldSession`:** `impl LootState { fn op(&mut self,
   cx: LootCx<'_>, ...) }`, donde `LootCx` es un struct de préstamos disjuntos por dominio
   (p. ej. `{ items: &mut ItemsState, map: &MapHandle, core: &SessionCore }`), no un contexto
   universal.
3. **Registro invertido como objetivo de F5:** cada dominio contribuye sus handlers y
   `world-server` los compone. La API exacta sigue pendiente del diseño de F5:
   `PacketHandlerFn` recibe hoy `&mut WorldSession` y
   `&SessionHandlerCatalogsLikeCpp` (`session/registry.rs`).
   No mover ese contrato a un dominio creando una dependencia de vuelta a `wow-world`.
   F5 debe definir el contrato bajo, la construcción de contextos y los adaptadores;
   `PacketHandlerEntry` seguirá siendo la única fuente de registro, admisión y llamada.
4. **DAG por capas sin ciclos**, según llamadas entrantes:
   - base: `player_binding`, `publication`, `canonical_access`, `player_items`, `instances`,
     `spell_state` (orden propuesto por la herramienta: entrantes − salientes);
   - medio: `world_entities`, `movement`, `quest`, `social`, `progression`, `combat`, `pets`,
     `persistence`;
   - cima: `spell_effects`, `loot`, `group`, `character`.

   La herramienta de acoplamiento re-deriva este orden antes de cada dominio.

Modelo de referencia: patrón común de rustc (`provide`), axum (`FromRef`), Bevy (`Plugin`) y
rust-analyzer (capas con invariantes probados). TrinityCore/AzerothCore son solo referencia de
**comportamiento**.

## 6. Fases y trabajo restante, en orden

F0–F3 son preparación estructural integrada; las cadenas y thunks restantes forman parte de F5.
F4 puede extraer aproximadamente 48k de las ~200k líneas actuales: no basta para alcanzar el
objetivo final de 20k–40k. Las estimaciones de cortes se remiden en la base de cada fase.

### F4a P3 — aceptación completada

Rama `1241-f4a-p3-pure-helpers`: movimiento `6f0660ab0`, corrección de tooling y candidato
aceptado `002ff5e462ba2c5c948934a270e67b137b456fcf` (evidencia en §8).
Mueve helpers del hub a `session/state/{hub_support,driver_phase}.rs`.
La campaña de §8 reemplaza los resultados de implementación del relevo como evidencia de
aceptación. La publicación/integración se tramita bajo la autoridad vigente y se registra en #1263.
No utilizar el helper local para decidir aceptación o merge hasta cumplir §11; se pueden
ejecutar directamente los comandos canónicos, conservando su evidencia y códigos reales.

### F4a P4a — dependencias base de `wow-world-core`

**Aceptación completada — 2026-10-02:** rama `1263-f4a-p4a-world-core`, base integrada
`1f8a7c800` (P3, PR #1264), candidato `d9c9e3637`. Extracción y consumidores aceptados:
catálogos, map manager legado, directorio, buzón independiente del pump, mascotas de cuenta,
persistencia de loot, políticas y helpers canónicos/phasing; tests/scanners y evidencia en §8.
La dev-dependency de wow-world activa `wow-world-core/test-fixtures`, sin self-dependency;
el binario de producción no activa fixtures/test-support. La excepción de visibilidad queda
acotada por los consumidores reales y AGENTS.md/develop-rustycore. P4b permanece pendiente.

- Crear `crates/wow-world-core`, `publish = false`, con las piezas que no nombran
  `WorldSession` ni los grupos hub: `map_manager/**` legado, `session/directory`,
  `session/mailbox/{durable,protocol,session_phase_permit,session_phase_rail}`,
  `battle_pet_account`, `catalogs/**`, `loot_persistence`, `session_policy` y sus piezas
  auxiliares. Estimación heredada: ~16.4k líneas, 53 ficheros completos y 41 cortes.
- Conservar montajes y rutas lógicas con una fachada deliberada y un `prelude.rs` acotado;
  `wow-world` depende del nuevo crate y reexporta solo las rutas de compatibilidad necesarias.
- Trasladar los tests a `wow-world-core/unit_tests/`, con rutas espejo. La lista compilada
  de P3 en `6f0660ab0`, con código Rust conservado en la base integrada `1f8a7c800`,
  contiene 187 unitarios en las familias seleccionadas: map_manager 139, battle_pet_account
  24, loot_persistence 5, mailbox 9, permisos de fase 5 y fixtures del directorio 5;
  además se traslada un doctest del directorio. Sustituye la estimación heredada de 173,
  con ejecución y aceptación en el destino P4a registradas en §8.
  Comparar el conjunto combinado de tests de origen y destino, conforme a §10; no exigir
  que `wow-world` conserve por sí solo los 3.950 tests.
- Definir y propagar `test-fixtures` a `wow-world-core` y `wow-session/test-support`.
  La dev-dependency de wow-world hacia wow-world-core activa los helpers trasladados para
  los tests del consumidor; `wow-world/test-fixtures` propaga la feature explícita al core.
  El grafo efectivo y la composición de producción sin fixtures se comprobaron en §8,
  sin depender de la antigua hipótesis de una self dev-dependency.
- Ampliar la cobertura del scanner antes de aceptar la extracción. El collector actual
  (`session_ownership/state_3.rs::collect_repository_baseline_with_persistence`) carga
  explícitamente `wow-world`, `wow-world-core`, `world-server`, `wow-network` y `wow-social`
  tras P4a. La extracción amplía `PackageRole`, selección de módulos,
  resolución de símbolos/reexports y gates de mailbox/directory/loot_persistence según sus
  consumidores. Las 17 regresiones nuevas prueban detección en el destino y resolución
  de fachadas/imports; no se limitan a cambiar constantes en `state_1.rs`/`state_2.rs`.
- Revisar `dependency-policy.json`, workspace/lockfile, filas físicas y el delta del
  inventario exhaustivo de persistencia. La sintaxis `--syntax-only` y `final --architecture`
  no sustituyen ese inventario cuando cambian sus entradas. No borrar baselines para
  ocultar código que dejó de ser inspeccionado.
- R1 debe pasar y el diff debe borrar cada origen trasladado. El margen heredado de unas
  550 líneas es una estimación, no aceptación ni autorización para cambiar la tolerancia.

### F4a P4b — hub en `wow-world-core`

Mover `SessionCore` y sus structs anidados, `HubRef`/`HubMut`, `SessionCatalogs`,
`SessionWorldConfig`, `SessionFixtures`, sus 11 estados de fixture y `session/appearance.rs`.
Estimación heredada: ~16.1k líneas y 293 bloques impl en unos 117 ficheros.
Incluir los impls del hub todavía alojados en handlers, por ejemplo
`handlers/character/condition_objects.rs` y `handlers/loot/requests/{context,generation}.rs`.

Los builders `hub_*`/`split_*`/`cx_*` que nombran `WorldSession` y los Cx correspondientes
permanecen en `wow-world` durante este corte. No afirmar que sus impls han salido del crate.

Cierre inmediato observado en `d9c9e3637`: el campo bootstrap de SessionCore también
requiere `PlayerIdentityBootstrapLikeCpp` de `session/player_binding.rs`. SessionCatalogs
nombra `WaypointPathResolverLikeCpp` (`session/map_admission.rs`),
`PlayerBootstrapCatalogTestFixtureLikeCpp` (`session/test_support/test_fixtures.rs`) y
`ObjectMgrCatalogsLikeCpp` (`session/catalog_capabilities.rs`). Verificar sus consumidores
al cortar las definiciones, sin crear una arista de vuelta Core → World. Los campos
ResetSchedule/InstanceLockMgr y el ModuleRegistry de fixtures requieren revisar las aristas
wow-instances y wow-module-api y la propagación de features antes de implementar P4b;
en aquel candidato aún no estaban en el manifiesto Core. Esta lectura preparó el
corte, sin aceptarlo.

**Corte P4b en curso — 2026-10-02, base `24a513855`:** trasladar también
`state/driver_phase.rs` y `state/hub_support.rs`, creados en P3 después del inventario
inicial. `SessionDirectory` contiene tres referencias de registro/canal; las fixtures
sociales vecinas pertenecen a `SessionSocialLimits` y no entran en este corte. Los
campos de transporte justifican las dependencias inferiores `wow-session` y
`wow-network`; los helpers de condiciones requieren `wow-conditions`. No crear una
dependencia Core → World ni trasladar el driver o sus builders.

El censo léxico inicial de implementación, después de extraer los inicializadores, registró
272 bloques objetivo en 527 fuentes Rust: 68 `HubMut`, 74 `HubRef`, 65 `SessionCatalogs`,
34 `SessionCore` y 31 de los otros tipos; diez están en handlers. Incluye los tres
`impl Default` añadidos para catálogos/configuración/fixtures y el nuevo constructor
Core. Sustituye la estimación de 293 para preparar los cortes, no prueba la resolución
de imports ni la paridad. `f4_hub_extract.py plan` conserva rutas físicas como hints;
la aceptación deberá comprobar los montajes lógicos reales y el conjunto trasladado.

Checkpoint local `4c2179c82`, rama `1263-f4a-p4b-hub`: inicializadores propios,
helpers del hub, seis tipos auxiliares y estado/fuente de sincronización temporal;
fachadas World conservadas. **NO VALIDADO:** no se ejecutaron Cargo, tests, formato
ni campaña de aceptación. El hub principal y su cierre de dependencias siguen
pendientes; este commit no se publicó ni acredita finalización de P4b. El scanner y
los codemods se guardaron después en el checkpoint descrito abajo.

El checkpoint local posterior `6dea7d06b` conserva la definición y el constructor de
`SessionCore`, sus siete estados anidados y el registro de fases a Core, conservando
las fachadas World. También salieron `MMapRuntimeConfigLikeCpp`,
`WaypointPathResolverLikeCpp`, `ObjectMgrCatalogsLikeCpp` y la fixture de catálogos
bootstrap. `SessionCatalogs` y `SessionWorldConfig` conservan sus inicializadores;
la configuración incluye ahora su cierre de aggro y selección de dificultad.
Los impls de conexión, identidad de conexión, sincronización temporal y acceso
canónico al Player se trasladan junto con sus dependencias. Los estados de fixture
de identidad, colecciones, auras y combate conservan sus gates y campos, sin nueva
autoridad. Incluye la resolución del mapa canónico y los tipos/helpers de snapshots
de poderes; los puertos de persistencia permanecen en World.
El manifiesto, la política y el lock incorporan las aristas inferiores de
transporte/instancias/configuración; `wow-module-api` y `wow-script` quedan opcionales
para `test-fixtures` y como dependencias de desarrollo, con propagación de
`wow-session/test-support`.
**Trabajo local NO VALIDADO:** los impls restantes todavía se están extrayendo;
la definición en Core por sí sola no establece compilación ni cierre de P4b.

El cierre de publicación requiere también `entity_update_bridge`: los impls de
Core llaman a `player_values_update_to_update_object`, cuya conversión usa los
helpers privados de los dos módulos del puente. El movimiento local del módulo
completo, guardado en `a7cb324bb`, conserva sus divisiones físicas y la fachada World; sus proveedores son
`wow-entities`, `wow-packet`, `wow-data` y `wow-core`. La revisión del diff constató
que sus tres archivos de código y tres de pruebas conservan el contenido byte a
byte; contienen 33 declaraciones de test, todavía sin ejecutar. No se añadió una
llamada de vuelta a World ni se duplicó la conversión. **NO VALIDADO:** conservar
el conjunto ejecutado de tests y los resultados de serialización sigue siendo una
obligación de la aceptación de P4b.

El checkpoint local `bfabaaf39` trasladó las definiciones de los once
grupos de fixtures y sus DTOs necesarios. Se conserva cada gate, el orden de campos
y los defaults de skills/rest/battle pets; las fachadas usan los reexports públicos
de Core, sin abrir sus módulos privados. Se corrigió en revisión el gate del módulo
de combate y se retuvieron los imports que todavía consume el inicializador World
de `SessionFixtures`. También se trasladaron los impls Core de admisión, drenaje de
comandos, publicación, actualizaciones de combate y acceso a dificultad/posición/
farsight/auras, con sus consumidores. La revisión distinguió el `ChatMsg` de
`wow-packet` del homónimo en `wow-constants`; se conserva el primero en los paquetes.
El checkpoint local `e4c379ec2` incorpora el agregado `SessionFixtures` (inicializador byte a
byte), `HubRef`/`HubMut` y su préstamo `shared`; los builders que toman `WorldSession`
siguen en World. También se trasladan la política de soporte y sus dos mensajes de
estado, y el adaptador canónico de criaturas con los métodos de sincronización y
rebinding de loot, conservando los cuerpos y el orden de locks/publicación. Los
imports y montajes se revisan junto a sus consumidores, incluidos los de pruebas.
El lote posterior incluye los accesos al jugador, barras de acción, perfiles CUF,
cinemáticas, publicaciones del hub y colecciones; sus cuerpos conservan el orden y
las ramas de fixtures. Los accesores de catálogos y política runtime cruzan el mismo
límite. El método Core de estacionalidad LFG, antes alojado en un handler, pasa a
`session/instances/lfg.rs` conservando su fallback y sin alterar los registros.
La revisión corrigió un import requerido por las pruebas de soporte y la visibilidad
del getter de exploración usado desde el shim World. La constante WAR_MODE mantiene
su consumidor de test; la de alcance de combate queda privada en Core.
El checkpoint local `21159a71c` traslada los impls del hub de presentación/vitales del jugador,
los de poderes/salud y los accesos de catálogos de criaturas, gameobjects, moneda y
zona/área. La conversión `const fn` de poderes conserva sus 25 ramas y su fallback a
Mana; las constantes de estado líquido conservan la API World mediante fachadas.
La comparación textual de los 20 bloques de esos siete archivos, normalizando solo
la visibilidad necesaria al cruzar crates, no encontró cambios de cuerpo. Es revisión
de fuente, no evidencia de aceptación. El acceso de spawn cierra también su dependencia
de los helpers `InitDbPhaseShift`/`InitDbVisibleMapId` alojados en `phasing`, con una
única definición y las fachadas World; su traslado no se dedujo del corte anterior.
Los tres helpers trasladados conservan sus cuerpos; sus anclas C++ son
`src/server/game/Phasing/PhasingHandler.cpp:47,528,564`, checkout
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`. Este movimiento conserva también los
filtros de datos existentes en Rust; no declara nueva paridad de esos filtros.
Otros nueve bloques de catálogos de spells/pets, chat y contactos conservan sus
cuerpos y gates, con montajes privados en Core y los shims en World.
El mismo lote traslada las vistas del hub de combate, reacciones de facción y auras,
y los accesos de configuración/catálogos de dificultad, loot, visibilidad y grupos.
Los DTOs de reacción, escalado y política de invitación tienen una sola definición
en Core, con fachadas World. Los helpers de inicio de combate conservan el orden
de lecturas, escrituras y liberación del guard; los broadcasts de spells mantienen
su destinatario y su orden. La comparación de cuerpos no encontró cambios en los
14 bloques de combate ni en los 43 métodos del último lote de auras, facciones y
accesos de configuración. Las dependencias del hub aún alojadas en World se cierran
en los lotes siguientes, sin stubs ni nuevos propietarios de estado.
El checkpoint local `6621390e0` traslada los catálogos de spells y objetos, los accesos de NPCs,
las operaciones del hub de reputación, descanso y talentos, y las proyecciones de
traits. Los DTOs de bootstrap y reputación mantienen una sola definición en Core;
las listas de reputación conservan sus consumidores de pruebas mediante fachadas
gated. La revisión de fuente conserva los cuerpos de los 15 bloques de objetos,
los ocho de reputación, los cinco de los adaptadores de progresión, los dos de
descanso y los seis de talentos, normalizando solo visibilidad y espacios. También
conserva los tres bloques de stand-state, los dos de traits y el getter de
tombstones de habilidades; los tests y adaptadores del shell permanecen en World.
Las visibilidades se contrastan con consumidores de `src`, `unit_tests` y `tests`,
incluidos los resets de talentos existentes: su callback compartido no acredita
la separación de orquestación pendiente en F5. Core añade las dependencias directas
ya existentes en el workspace `num-traits`, `wow-ai` y `wow-progression`; la revisión
de sus manifiestos no encuentra una dependencia de vuelta a Core. Estos traslados
incluyen también los catálogos compartidos de progresión y valoración, sus fachadas
y defaults, y los cuatro bloques del hub de XP/escalado. La valoración y los
encantamientos conservan sus tres bloques, incluido el comportamiento existente
`standard_price = false`. Los seis bloques de habilidades y el de habilidades
iniciales conservan la autoridad de slots/tombstones y el orden de publicación;
sus cinco conversiones auxiliares tienen una sola definición en Core. El cierre de
imports/montajes sigue sin aceptación, publicación o QA live.
El checkpoint local `86d843930` mueve los ocho bloques del hub/catálogos que seguían en los
handlers de condiciones, stats, mail de login, emote, loot y persistencia de quest
status a hojas privadas de Core. Sus cuerpos se conservan; la firma de quest status
usa el mismo tipo nominal `wow_entities::PlayerQuestStatusRecord` de la fachada
World. Mail conserva el await anterior a la instalación del estado y sus ramas de
fallo; emote conserva el envío propio anterior al broadcast. Los once bloques de
estado/protocolo/validación de movimiento conservan los cuerpos y gates, la
precedencia legacy/canónica de Creature/Pet, el fallback del reloj y el contador
wrapping. El DTO de pertenencia a transporte, tres helpers de opcodes y la tabla
de velocidades quedan definidos una sola vez en Core con fachadas World. La
revisión corrigió un import de fachada copiado a Core y restituyó el DTO del ACK
de velocidad en World. Las anclas contrastadas son `Unit::SetSpeedRate` y
`Unit::UpdateSpeed` en `src/server/game/Entities/Unit/Unit.cpp:8294,8460`, y las
ramas de mover/transporte de `src/server/game/Handlers/MovementHandler.cpp`,
checkout `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`; este traslado no acredita
nueva paridad de los adaptadores ni retirada del runtime legado. Tampoco inicia
la aceptación de P4b.
La continuación de mascotas conserva los diez bloques de catálogos, hub y fixtures
de pet/summoning, la prueba de fuente vacía de `character_pet` y la solicitud de
unsummon temporal. El DTO de filas del stable mantiene sus 16 campos y derive; los
dos decodificadores mantienen sus ramas y defaults, con fachadas World. La revisión
de fuente conserva los cuerpos, los callbacks de una sola ejecución, el tratamiento
del handle obsoleto y la liberación de guards antes de enviar o difundir paquetes.
El traslado conserva los límites de persistencia del adaptador Rust; no acredita
una nueva implementación de `Pet::SavePetToDB`. Referencia contrastada:
`Player::RemovePet`, `src/server/game/Entities/Player/Player.cpp:20863`, del mismo
checkout versionado. El censo de continuación incluye también los bloques del hub de
`battle_pet_purchase/ops_1.rs` y `player/quest_persistence_projection.rs`, fuera
de `session` y `handlers`; su cierre sigue dentro de P4b.
La continuación revisada mueve los 19 bloques de velocidad, caída, publicación,
transferencia y avance de spline a hojas privadas de Core, conservando firmas,
cuerpos y gates. Se corrigió antes de aceptar un error mecánico que había borrado
nombres de métodos. Los 14 bloques originales de catálogos de Creature, amenaza,
battleground, XP, talentos, moneda y misiones conservan sus operaciones; el bloque
de catálogos de Creature queda repartido en dos impls sin cambiar sus tres métodos.
Los dos DTO de creación, el helper de máscara de clase y el decoder de queue ID
tienen una sola definición con fachadas World. La proyección de quest status usa
el mismo tipo nominal de la fachada anterior.
Los 12 bloques de battle pets conservan la selección de trainer, slots, journal,
flags y publicación. La revisión retiró tres bloques duplicados en la hoja de
destino y mantiene el límite de persistencia del dueño de cuenta. Conserva también
la divergencia Rust ya documentada para filas Removed durante heal; este traslado
no la repara ni reclama paridad nueva. Referencias contrastadas: `BattlePetMgr`
en `src/server/game/BattlePets/BattlePetMgr.cpp:584,799,890,900` y
`src/server/game/Handlers/BattlePetHandler.cpp:38,96`, del mismo checkout versionado.
Taxi conserva los cinco bloques y sus 38 métodos, las cuatro conversiones fixture
y los valores de sus constantes. Mantiene la precedencia canónica, el rechazo de
handles obsoletos, el filtro de GUID vacío, la instalación y retirada del vehículo,
y el orden del contador y de los dos paquetes de vehicle ID. Se corrigió el gate
de un import usado en producción. Referencias: `Player::CleanupAfterTaxiFlight`,
`src/server/game/Entities/Player/Player.cpp:22019`, y `Unit::CreateVehicleKit` /
`Unit::SendSetVehicleRecId`, `src/server/game/Entities/Unit/Unit.cpp:11298,13031`.
El censo por fuente ya no encuentra impls de HubRef/HubMut/SessionCore/SessionCatalogs
en World; los substates de dominio y su orquestación siguen pendientes de F4b/F5.
La revisión por fuente de imports, montajes y consumidores del lote está cerrada;
incluye las fachadas de teleport, los gates de fall/battleground y los helpers
internos de mascotas con visibilidad limitada a sus consumidores Core.
**NO VALIDADO:** falta reconciliar los metadatos y baselines revisados y ejecutar
la aceptación completa. La inspección de cuerpos no sustituye compilación, tests,
inventario ni evidencia de producción.
El reinicio del host borró los listados P4a de `/tmp`. Antes de compilar P4b se
conservaron cinco binarios de test y sus fingerprints de Cargo en
`target/validation-v2/evidence/p4b-baseline-retained-20261002/`, con hashes y
proveniencia. Su asociación al manifiesto P4a es una inferencia apoyada en targets,
features, fechas y ausencia de builds P4b; el binario no certifica un SHA. Todavía
no se ejecutaron sus listados. La comparación deberá recuperar sus identidades,
cotejar las fuentes de la base y registrar esta limitación sin atribuir a P4b
resultados de una ejecución anterior.

**Primera aceptación P4b, candidato `5a0e68293` — 2026-10-02:** árbol limpio;
`final --base origin/3.4.3 --architecture --timings --logs --keep-going`, un job,
manifiesto `target/validation-v2/manifests/20261002T212119.987775Z-33228-final.json`.
Resultado **FAILED**, exit 1, duración 501,887 s; `verify --require-profile final`
rechaza ese manifiesto. Inicio 21:21:19,987 UTC y fin 21:29:41,874 UTC. El primer
listado requerido comenzó a las 21:21:09,598 UTC y la verificación posterior se
registró a las 21:31:12 UTC; la campaña completa sigue pendiente, con reparaciones
medidas aparte. Los listados retenidos recuperan 3.949 casos y un ignored; falta
compararlos con el candidato y añadir el doctest. Los 33 casos del bridge son
R100 y cambian solo de crate; no hay otras altas/bajas World/Core por fuente.
El checker ejecutó 406 pruebas: 400 PASS y seis FAIL; World/Core no llegaron a
ejecutar sus suites porque Core tiene tres errores de imports/dependencias.
Los fallos incluyen dos techos físicos de roots, dos fachadas glob que impiden
inventariar registros, diferencias de formato y tres regresiones nuevas de
fixtures/diagnóstico; los otros tres fallos de tests comparten el bloqueo glob.
El baseline nominal nuevo y los extras de producción/configuración/inventario
todavía no están aceptados. R1 da S=20.503, G=24.270, tolerancia 21.828,2 y exceso
2.441,8: Core producción añade 22.194 líneas netas y el checker 2.076; los 1.026
test LOC trasladados se cancelan. La revisión no encontró una copia de cuerpos
o del inicializador de fixtures que justifique retirarlos. R1 sigue abierto;
no se rebaja el gate ni se elimina cobertura para publicar este corte.
Las reparaciones revisadas corrigen los proveedores de reputación y trasladan la
dependencia que faltaba de consultas de aura: dos DTO, cuatro funciones y su
conversión conservan campos, derives y cuerpos; World mantiene sus aliases y
Core una sola implementación. El porcentaje de autoataque conserva su pliegue
en un helper privado junto a su único consumidor. Las fachadas glob pasan a
exports nombrados: el bridge conserva sus 27 funciones públicas y hub_support
sus ocho items. El checker corrige el montaje de una fixture, conserva los
rechazos de ausencia/duplicidad con diagnósticos precisos y limita el resolver
singular a tests. Los dos techos físicos se ajustan solo a sus siete líneas
necesarias de montaje, sin margen ni cambio de la observación histórica.
La revisión corrigió además un comentario heredado: C++ usa `forward_list`
(`Unit.h:631`), `push_front` (`Unit.cpp:3567`) y devuelve `m_modAuras[type]`
(`Unit.h:1284`); el traslado mantiene el orden Rust por slots ascendentes, sin
resolver esa diferencia. Es inspección de cuerpos y evidencia versionada, no
una nueva prueba de paridad. Estas reparaciones todavía necesitan aceptación.
Los dos comandos de formato terminaron con exit 0; la revisión del delta conserva
los cuerpos y cfg, con los roots del checker en 58 y 71 líneas. La generación
del baseline de sintaxis terminó con exit 0 en 217,528 s (21:49:59–21:53:36 UTC),
sobre `5a0e68293` con las reparaciones dirty; su checkpoint posterior es `4ac154416`.
Es diagnóstico de reparación, no aceptación final. La delta revisada conserva los
84 bridges y los 3.204 items WorldSession; registros 645→648 por traslados/imports,
inputs 57→58 por el atributo inline del driver ya existente. El nuevo dueño nominal
registra 36 campos ordenados, 34 impls/127 items y un proveedor Core único para
`WorldSession.core`. Solo se sustituyó ese subárbol revisado; los baselines de
persistencia todavía requieren contraste. R1 tras formato mantiene base, ratio y
slack: S=20.696, G=24.170, tolerancia 22.030,8, exceso 2.139,2, exit 1; el corte
sigue sin aceptación para publicación.

**Repetición tras reparación, `c8c7443f1` — 2026-10-02:** árbol limpio;
manifiesto `target/validation-v2/manifests/20261002T215924.304069Z-38677-final.json`,
21:59:24,303–22:06:58,430 UTC, 454,127 s, **FAILED**, exit 1; `verify` lo rechaza.
Arquitectura/fixtures, sintaxis ownership, formato e higiene pasan. R1 conserva su
fallo; World/Core no ejecutan suites por un acceso del shell al contador anti-flood
privado de Core. El checker pasa 405/406: la fixture de proveedor ausente sigue
siendo rechazada por el scanner de bridges antes del diagnóstico nominal esperado.
Se corrigen esas dos fronteras y se contrastan warnings por configuración; no se
rebaja la validación ni se acredita cierre de P4b. El corte no exige publicación
propia antes del trabajo local F4b: se continúa la secuencia dentro de #1263 cuando
su dependencia técnica de Core tenga evidencia, conservando R1 para publicación
y la revisión explícita del diseño F5. Extras/inventario/campaña completa pendientes.
La segunda reparación mantiene privado el mapa de AntiDOS: Core devuelve solo
el contador mutable del opcode solicitado y World conserva instante, incremento
y políticas (`WorldSession::DosProtection::EvaluateOpcode`, C++ 1251–1270).
La fixture usa un tipo no suministrado sin alias/ruta inválida, conservando ambos
asserts y el rechazo; se elimina un import Core cuya llamada ya era cualificada.
El checkpoint local `d58a5dffd` conserva estas correcciones sin aceptación.
La comprobación afectada `cargo check --locked --workspace --all-targets --jobs 1 --timings`
terminó con exit 101 en 499,774 s, 22:23:03,711–22:31:23,485 UTC, sobre ese SHA
con árbol limpio. La biblioteca World compiló y su fingerprint confirma
`test-fixtures`; cinco referencias sin import impiden compilar sus unitarios
(status de quest, generador de void storage, flags/GUID de presentación e
inicialización de phasing). No se ejecutaron esas suites. Se reparan sus imports
y se contrasta la delta de warnings con la configuración retenida equivalente.
Los registros están en `target/validation-v2/evidence/p4b-acceptance-20261002/repair-diagnostics/`.
La campaña completa continúa abierta; estos tiempos no cumplen el presupuesto
total de 600 s ni sustituyen los extras de aceptación pendientes.
La comparación de warnings de la biblioteca Core con `test-fixtures` no encontró
mensajes nuevos: tres ya estaban en Core y cinco conservan el mensaje y el código
señalado en el bootstrap trasladado desde World. La asociación histórica del
artefacto a P4a es inferida, no una atestación de SHA; el contraste no acredita
los unitarios ni la configuración de producción sin fixtures. La reparación de
imports World conserva los proveedores de test y retira once fachadas internas
sin consumidores; los campos y cuerpos retenidos de `state.rs` coinciden con el
checkpoint anterior. La compilación y las suites del nuevo lote siguen pendientes.
El formato de este lote terminó con exit 0 en 9,944 s (22:48:48–22:48:58 UTC);
la revisión conserva los cuerpos y la cadena nominal de `WorldSession.core`.
Las fachadas retiradas no definen tipos de registro ni inputs generados: no se
modifican los baselines por esta limpieza sin comprobar antes su delta real.
El checkpoint local `99d86c533` guarda el lote revisado. Su comprobación afectada
del workspace terminó con exit 101 en 157,004 s, 22:50:04,200–22:52:41,204 UTC,
con árbol limpio: los cinco imports anteriores ya compilan; queda un import del
callback XP en el shim de progresión, retirado durante la limpieza. Se restaura
su proveedor Core mediante la fachada existente. El contraste por símbolo,
evitando diferencias de agrupación/ruta del diagnóstico, identifica los aliases
internos que quedaron huérfanos tras retirar sus bindings World. Se completa esa
cadena conservando los consumidores de tests; no se ejecutaron las suites ni se
acredita aceptación del nuevo lote. Los fingerprints y diagnósticos de ese SHA
se retuvieron antes de la siguiente compilación.
La revisión del cierre de aliases conserva los proveedores efectivos de los
shims y retira dos archivos World que solo reexportaban fixtures Core sin
consumidores. No cambia definiciones, campos ni cuerpos. El formato terminó
con exit 0 en 9,944 s (23:08:13–23:08:23 UTC); el lote permanece sin aceptación.
El checkpoint `67fd33466` pasa la comprobación completa del workspace y sus
targets en 108,580 s, 23:10:22,736–23:12:11,316 UTC, exit 0 y árbol limpio.
Los fingerprints confirman `test-fixtures` en la biblioteca y los unitarios de
World. Quedan siete aliases privados con warnings nuevos en sus fachadas; cuatro
atributos se limitan a `cfg(test)`, igual que sus consumidores. Esa delta está
revisada, no recompilada. Compilación no equivale a ejecución de suites: pruebas,
configuración de producción, inventario y gates de publicación siguen pendientes.
El candidato limpio `e5de2c450` vuelve a pasar el workspace `--all-targets`
en 42,623 s (23:17:31,758–23:18:14,381 UTC). La ejecución conjunta de las
bibliotecas World/Core termina con exit 101 en 257,058 s
(23:18:14,381–23:22:31,439 UTC): Core pasa 220 casos; World pasa 3.679,
falla dos comprobaciones textuales de login y conserva un caso ignorado.
Estas comprobaciones aún buscan el cuerpo de habilidades iniciales y correo
en sus antiguos archivos World. Se revisan contra los proveedores Core y la
delegación/orden del coordinador, sin retirar sus condiciones. Sus cuerpos
coinciden con la base P4a; este resultado no acredita paridad nueva de login.
Los listados ejecutables conservan la unión de identidades de las dos bibliotecas:
33 casos pasan de World a Core con el mismo nombre, sin altas/bajas ni cambio
del caso ignorado. World ahora activa `test-fixtures`; la base World retenida
no la activaba, por lo que este contraste no sustituye la configuración de
producción. Los registros y la comparación están en
`technical-e5de2c450/` dentro de la evidencia de esta campaña. La secuencia se
detiene antes del scanner y sus suites release; esos pasos siguen pendientes.
La reparación de las dos comprobaciones conserva todos sus marcadores y añade
la delegación exacta de argumentos/resultado desde World; únicamente cambia el
archivo que contiene el cuerpo comprobado. La comparación de avisos unitarios
World/Core con los artefactos retenidos de `test-fixtures` no encuentra nuevas
identidades: los cinco avisos de bootstrap conservan mensaje y texto señalado
al pasar a Core. Se conserva la limitación de procedencia histórica: el perfil
del fingerprint difiere y su comando original no está atestado. Esto no acredita
equivalencia exacta de perfil ni convierte el resultado rojo en aceptación.
El candidato limpio `9ce2f28da` repite las bibliotecas con la misma selección
World/Core y pasa en 53,890 s: 3.681 casos World, 220 Core y el mismo caso
ignorado. La sintaxis ownership pasa en 45,834 s; las 406 regresiones del
checker pasan en release en 245,622 s, incluidos 48,50 s de ejecución.
La secuencia se registra en `technical-9ce2f28da/sequence.json`.
La comparación de perfiles se acota: Core tiene un artefacto histórico unitario
equivalente, y World tiene un check unitario equivalente cuyos avisos coinciden
con los del ejecutable actual. No existe un ejecutable World histórico con
esa misma feature; la asociación a SHA/comando de la base sigue siendo inferida.
Los extras de `9ce2f28da` pasan `cargo check --locked -p world-server --bin
world-server --jobs 1 --timings --message-format=json` en 231,754 s y Core
predeterminado `--all-targets` en 8,542 s. Los artefactos del binario prueban
features vacías en World/Core/Session. El contraste equivalente de avisos de
producción encuentra cuatro imports nuevos de Core usados solo por fixtures;
se alinean con el cfg de sus consumidores, conservando los cuerpos. El aviso
`unused_mut` de presentación conserva la identidad y código trasladados de World.
La suite de world-server termina roja en 316,235 s: 596 PASS y un FAIL textual
que aún busca el catálogo de encuentros en World. Su reparación conserva carga,
instalación y tipo del catálogo, añade el accessor prestado Core y verifica
el setter World. Se contrasta `DB2Stores.cpp:124,699` y
`Player.cpp:20725–20744` de `a5f8da2eb`; no se cambia esa operación.
Los registros y artefactos están en `extras-9ce2f28da/`; la secuencia se detuvo
antes de integraciones, doctest y herramientas. La nueva delta requiere repetir
su evidencia afectada. Inventario exhaustivo y publicación siguen pendientes;
las ejecuciones previas conservan sus SHA reales y no se relabelan.
El checkpoint limpio `695c27aba` pasa producción en 17,010 s, Core
predeterminado en 2,720 s, World/Core lib en 64,969 s (3.681/220 PASS y
un ignorado) y world-server lib en 36,259 s (597 PASS). La configuración
equivalente de producción conserva cero avisos nuevos tras retirar los cuatro
imports de fixtures; se mantiene la limitación de procedencia histórica.
La integración falla en 21,717 s: renombrado pasa sus 12 casos y registro su
caso; login/owner pasa uno y falla 33 en la hidratación inicial compartida.
La traza identifica un desajuste del puente: World biblioteca activa
`test-fixtures`, los lectores Core de identidad usan esa feature, pero cinco
bloques existentes que escriben esos campos Core siguen bajo `cfg(test)` en
World. El nombre de fixture vacío corta `build_initial_player_for_owner_like_cpp`
antes de instalar el handle; la hidratación de correo rechaza correctamente
el propietario ausente. Se alinean esos escritores con la disponibilidad de
sus campos, conservando los guards de propietario/instalación y los fixtures
todavía propios de World. No se debilita la resolución canónica ni las pruebas.
El contrato de orden se contrasta con `CharacterHandler.cpp:1061–1077` y
`Player.cpp:17759,18560` de `a5f8da2eb`. La secuencia está en
`affected-and-extras-695c27aba/sequence.json`; doctest, herramientas e inventario
no llegaron a ejecutarse. Esa regresión bloqueó el siguiente traslado local
hasta reparar y acreditar la integración; no hubo publicación ni QA live.

**Evidencia afectada, candidato limpio `3ea514530` — 2026-10-03:** los cinco
escritores conservan sus cuerpos y usan el cfg de los campos Core. Las 47
integraciones pasan en 27,481 s: renombrado 12, registro 1 y login/owner 34,
incluido el rechazo sin PlayerManager. World/Core lib pasa en 37,976 s
(3.681/220 PASS y un ignorado); world-server lib pasa sus 597 casos en
32,948 s. Producción pasa en 15,835 s, con features vacías en
World/Core/Session, y el doctest Core pasa su caso en 20,177 s. Los avisos
de producción conservan las mismas identidades que `695c27aba`; dos JSON
World cambian únicamente offsets de bytes tras alargar atributos cfg.
La unión World/Core/integraciones conserva sus 3.949 identidades y el
mismo ignorado: los 33 traslados de tests World → Core siguen exactos.
Estas bibliotecas/integraciones activan `test-fixtures`; esa comparación
no sustituye el check separado de composición de producción. La asociación
histórica de artefactos a SHA sigue siendo inferida, no atestada.

Composición de módulos, sus 12 fixtures, los 87 casos iniciales de codemods y los
self-tests directory/mailbox pasan. Los planes Core, battle-pet visibility,
directory, mailbox y hub pasan. Tres planes detectan consumidores de QA
desactualizados: canonical espera el lector privado fuera de `cfg(test)`,
battle-pet espera los cfg anteriores de cuatro reexports DTO y phasing espera el contenido
Core exclusivo de P4a. Sus reparaciones deben conservar el reconocimiento
exacto y los rechazos de estados inesperados; no revertir los traslados ni
aceptar cualquier contenido residual. Los registros están en
`affected-and-extras-3ea514530/` y `remaining-plans-3ea514530-20261003/`, bajo
`target/validation-v2/evidence/p4b-acceptance-20261002/`.
El inventario exhaustivo comenzó a las 00:07:42 UTC sobre `3ea514530`, con
Rust, manifiestos y políticas congelados; las únicas ediciones permitidas
son la reparación Python canonical y este checkpoint, fuera de sus entradas.
El check completo termina **PASS** a las 00:20:32 UTC en 770,160 s, coste
exhaustivo separado: 7.718 filas de producción + 2.221 de fixtures,
1.034 grupos semánticos, 84 filas bridge, 648 filas de registro y 58 inputs
generados exactos. Compara también el snapshot de persistencia y su política
semántica; no se regeneró ninguno para aceptar el traslado. La huella de
Rust/manifiestos/políticas conserva cero cambios y HEAD sigue en `3ea514530`.
El registro está en `exhaustive-3ea514530-20261003/record.json`.
La reparación canonical pasa después sus 88 casos de codemods en 1,419 s y
el plan en 0,114 s, con las dos fuentes Python y este documento sucios sobre
`3ea514530`; se guardan sus hashes en `canonical-repair-3ea514530-20261003/`.
No se atribuye esa delta sin commit al candidato limpio de las suites Rust.
Las reparaciones Python completas pasan sus **91 tests** en 1,768 s y los
ocho planes Core/canonical/battle-pet/visibility/phasing/directory/mailbox/hub,
además de whitespace. El recognizer battle-pet conserva P4a y exige los cuatro
reexports P4b bajo `cfg(test)`, incluido Cage; phasing empareja cada fachada
con su residual Core completo y rechaza cuerpos/imports/constantes adicionales
o alterados. Los hashes y estado sucio de las seis fuentes Python sobre
`3ea514530` están en `tool-repairs-3ea514530-20261003/sequence.json`; Rust,
manifiestos y políticas son el mismo candidato acreditado por las suites y
el inventario. Esa evidencia satisface la dependencia técnica local de Core
para continuar F4b, conservando los gates de publicación.

El diagnóstico R1 posterior a las reparaciones Rust conserva base
`24a513855`, ratio 5% y slack 300: **S=20.883, G=24.212, allowance=22.227,15,
exceso=1.984,85, exit 1**. Se registra en
`tool-repairs-3ea514530-20261003/net-move.record.json` y su salida JSON.
La campaña ordinaria conserva el incumplimiento de 600 s ya registrado;
R1 sigue siendo un gate de publicación pendiente, sin cierre de P4b/#1263.

La construcción debe conservar el orden exacto de expresiones, RNG, relojes, canales
y campos. Extraer primero el literal de `SessionCore` a una inicialización propia;
los rails y la conexión con su endpoint siguen construyéndose en `WorldSession::new`.
Los literales de catálogos/configuración/fixtures necesitan el mismo tratamiento al
cruzar la frontera. La feature de fixtures puede estar activa en Core cuando World
compila como biblioteca sin `cfg(test)`: un constructor resuelve los campos internos,
pero no puede inventar el préstamo `HubRef::fixtures` de un campo ausente en World.
Para ese préstamo, P4b añade una dev-dependency de World sobre sí mismo con
`test-fixtures`, conservando la propagación a Core/Session. No importar el self-crate
en los unitarios ni mezclar sus identidades con las del crate compilado para tests.
La decisión se limita a la composición de pruebas; comprobar durante la aceptación
los targets unitarios/integración y el grafo de producción sin fixtures. Esa
composición pasa en `3ea514530`; no acredita los gates restantes de P4b.
La solución de P4a mediante dev-dependency de Core no probaba este nuevo
caso de P4b, por lo que su resultado sin self-dependency no se extrapola a las vistas.

El checkpoint local `e047d2336` incorpora el scanner y los codemods de preparación,
todavía **NO VALIDADOS** y sin publicar. El scanner conserva su superficie de `WorldSession` y añade una superficie
separada de `SessionCore`: definición, campos e impls con paquete, módulo y cfg,
incluida la resolución del campo `WorldSession.core` a través de las fachadas.
Resuelve los proveedores nominales en una consulta por lote, incluidos aliases
renombrados y `self as Alias`; el ordinal de cada campo conserva el orden de
declaración en la superficie comparada. Las regresiones escritas cubren proveedores
ausentes/ambiguos/ajenos, aliases, homónimos, eliminación de impls y reordenación de
campos. No basta con conservar el texto `SessionCore` en el baseline. Se mantienen
los rechazos de definiciones duplicadas/ajenas y de `WorldSession` alojado en Core.
Pruebas, reconciliación del baseline y aceptación siguen pendientes; este checkpoint
no es evidencia verde ni cierre de fase.

Preservar orden de drop, hooks, locks, cancelación y persistencia. Medir la deuda de 56 warnings
informada en P2 para builds con feature: hacer público un item no garantiza eliminar todos;
aceptar solo el delta revisado y ningún warning nuevo.

### F4b — crates de dominio

Orden propuesto por menor cierre de dependencias: social, spell_state, interaction, instances,
visibility; después loot, world_entities, inventory y lifecycle. Confirmar el cierre real
antes de cada extracción. Cada crate recibe el estado, los impls que no nombran
`WorldSession`/Cx residentes en el shell, y sus tests independientes de sesión.
Los impls todavía ligados al shell conservan un dueño explícito y se trasladan en F5;
no se duplican para aparentar una extracción completa.

Preparación social sobre `c8c7443f1`: los nueve impls de `SessionSocialLimits`
usan Hub Core, pero su cierre incluye addon-filter, throttle, DTOs y cuatro
fixtures hoy definidos en World, con consumidores en otros dominios/tests.
`wow-world-social` dependerá de Core y del `wow-social` inferior; reutilizar este
último crearía un ciclo. Conservar el literal RaF 85/4: `Default` da 0/0.
El contraste `a5f8da2eb` corrige una atribución heredada: `m_chatFloodData` vive
en C++ Player (`Player.h:2397–2410,2928`, `Player.cpp:20556–20598`), mientras Rust
lo guarda en sesión. El movimiento conserva ese comportamiento y no acredita
paridad de owner; la diferencia de lifetime permanece para la pista F6. Addon
registro/unregister conserva `WorldSession.cpp:948–976`, incluido limpiar la
lista sin alterar el flag. Ese checkpoint de preparación aún no implementaba F4b.
El contraste de proveedores en `99d86c533` confirma que `HubRef`/`HubMut` y
`SessionCommand`/payloads de duelo pertenecen ya a Core; no bloquean el traslado
de los nueve impls sociales. El cierre World real incluye las DTO de
calendar/petitions/duel, el evento de force-deselect, la distancia de XP de
grupo y las constantes de duelo/guild que comparten los
wrappers. Conservar sus fachadas necesarias, el cuerpo de flood con su `HubMut`
Core y el puente `test-fixtures` aprobado; los shells/builders siguen en World.
La revisión sobre `3ea514530` mantiene `GroupReconciliationOutcomeLikeCpp`
en World: solo lo consume su reconciliador `WorldSession`, no los impls del
estado social. El noveno impl sí está en `handlers/social.rs:751`, bajo el
alias `crate::session::SessionSocialLimits`; el `WorldSession` de contacts
permanece y adapta sus accesos RaF a la API del estado. No sustituir ese
impl de handlers por el shell de contacts en el censo de extracción.

**Implementación Social iniciada — 2026-10-03, checkpoint `d4899af77`:**
la dependencia técnica local de Core está acreditada en P4b; el corte Social
continúa en la misma rama, con R1/publicación pendientes. Se trasladan estado,
cuatro fixtures, DTOs y los nueve impls a `wow-world-social`; constructor,
lecturas/escrituras de límites, addons y consumidores de fixtures adaptan sus
accesos mediante una API acotada. Los campos de producción no se abren a World.
El bloque `chat/operations.rs:160` contiene away/emote; el antispam está en
`catalogs/operations.rs:482`, usando el `unix_now` de Core. Su GM seam y
aritmética saturating se conservan respecto del Rust actual, con la diferencia
frente a `Player::UpdateSpeakTime` explícitamente pendiente de F6.
Esta implementación sigue en nivel 1: cambios y pruebas escritos se distinguen
de la aceptación; todavía no hay evidencia aceptada del nuevo crate Social.
La revisión del corte conserva el constructor explícito 85/4 y el `Default`
0/0, el orden de campos/fixtures y sus defaults. Registro de addons mantiene
`extend(packet.prefixes)`, el límite y el logging del shell; unregister limpia
solo la lista. El fixture de comercio mantiene la constante original de
`wow_packet::packets::misc`, con su cast a `usize`. Los consumidores World de
calendario, comercio, hermandad, duelo y force-deselect se cierran mediante
operaciones acotadas; no se exponen los campos de producción ni los fixtures.
El scanner incorpora el rol separado `WorldSocial`, su raíz real y la resolución
nominal de aliases/reexports con prioridad del shadowing local. Diez regresiones
nuevas quedan escritas, conservando las 406 anteriores; no se regeneran baselines.
Anclas contrastadas en `a5f8da2eb`: `Player.cpp:23440,24870,25235`,
`DuelHandler.cpp:29,58,86`, `GuildHandler.cpp:63,70` y
`TradeHandler.cpp:575`, además de las anclas de chat/addons anteriores.
El traslado conserva las operaciones Rust y sus límites ya existentes, incluidos
los fallbacks de fixtures; no acredita nueva paridad de guild/duel ni ejecución
de suites, composición de producción, captures o QA live. Ese trabajo continúa
pendiente de la aceptación del macro y de la pista F6.
El checkpoint local `867885a6a` guarda el corte Social y el cierre revisado de
sus consumidores. La inspección por `rg` ya no encuentra accesos World a sus
campos privados ni los nueve impls en el origen. **NO VALIDADO:** no se ejecutó
Cargo, formato, suites o inventarios para ese candidato; no hubo publicación.

**SpellState implementado localmente — 2026-10-03, base `867885a6a`:** el cierre revisado
incluye 18 impls puros, 22 campos del estado y el fixture de 14 campos.
`wow-world-spell` recibe adquisición/spellbook, auras/shapeshift y cast/cooldown/
publicación; los wrappers ligados a `WorldSession`/Cx permanecen en World.
Los records de skills siguen en Core; los DTO de spells trasladados tendrán un
único proveedor y las fachadas necesarias. El constructor conserva las 22
expresiones y su orden, incluido offhand=true, sin sustituirlo por un Default.
El getter de casts de duelo sigue en el shell: Spell no requiere una arista Social.
Anclas de owner/operación revisadas en `a5f8da2eb`: `Player.h:1753–1755`,
`Unit.h:1417–1418`, `Player.cpp:23455–23500`, `SpellHistory.cpp:670,689`,
`Spell.cpp:4656,4765,5378`, `SpellAuraEffects.cpp:1792` y
`Unit.cpp:8213,9287`. Rust conserva por ahora el snapshot/resend de SpellHistory
frente al lector vivo C++; esa diferencia pertenece a F6. Los 18 impls ya se
retiraron de World. La comparación acotada de sus 102 métodos conserva los
cuerpos, salvo namespaces, los gates de fixtures y referencias al mismo
proveedor Core. `present_visual` tiene un solo proveedor en Spell; se conserva
su conversión de dos campos, con `CombatLogPacketsCommon.h:86–90` y
`.cpp:164–178` como procedencia del campo serializado `SpellXSpellVisualID`.
El cierre incluye también el DTO de spell focus que seguía definido en World.
Los consumidores y fachadas ya usan el nuevo proveedor; los accesos con nombres
similares que quedan en la configuración de criaturas pertenecen a otro estado.
Las tres pruebas puras de restricciones de trainer se trasladan, con su módulo
y aserciones intactos, a la hoja Spell. Sus helpers permanecen privados; los
escenarios ligados a sesión conservan sus montajes en World.
El workspace, lockfile y policy incorporan ocho dependencias internas reales y
`num-traits`/`tracing`, sin una arista World/Social. **NO VALIDADO:** esta
inspección no acredita compilación, suites ni aceptación del nuevo crate.

Preparación Interaction sobre `867885a6a`: hay cinco impls puros, no solo los
dos de `session/npc_interaction.rs` y `support_features.rs`. Incluye los aliases
de `handlers/character/{gossip.rs,bank.rs,vendor_admission.rs}`. El cierre contiene
el helper `vendor/rules.rs::vendor_buy_stock_refill_count`, los DTO
`VendorItemCount`/`VendorBuyItemTestOverrideLikeCpp`, cinco campos del estado y
el fixture Support con sus cinco defaults (solo support=true). Gossip y
PlayerInteractionData ya tienen proveedores inferiores; conservarlos y sus
fachadas. Anclas revisadas de `a5f8da2eb`: `GossipDef.h:222–236`,
`GossipDef.cpp:234,240,307`, `BankHandler.cpp:284`, `NPCHandler.cpp:126–128`
y `Creature.cpp:3038,3072`. La propiedad Rust del stock por sesión, frente al
Creature C++, y su guard de incr_time=0 permanecen como límites heredados de F6;
no repararlos dentro de este traslado. La implementación local y su montaje
están cerrados: estado privado, constructor explícito, los cinco impls y sus
24 métodos conservados, DTOs y consumidores. El test puro de refill se traslada
con las tres aserciones originales; el helper queda privado en Interaction.
El crate usa cuatro dependencias internas reales, sin dependencias externas.
Las raíces compartidas y constructores de World tuvieron un único implementador.
El scanner incorpora `WorldSpell`/`WorldInteraction`, sus mounts reales y ocho
regresiones escritas, sin modificar las 416 anteriores ni regenerar baselines.
**NO VALIDADO:** no se ejecutó Cargo, formato, suites o inventarios para el
conjunto Spell/Interaction. No hay nueva evidencia de producción ni publicación.
El checkpoint local `24c42d87d` guarda este cierre de ambos dominios, los cuatro
tests trasladados y la integración del scanner. Su estado sigue siendo
**NO VALIDADO**; no sustituye la aceptación pendiente del macro.

El censo siguiente encuentra ocho impls de `InstanceState` y tres de
`VisibilityState`, incluidos los aliases en handlers. El bloque de respec en
`handlers/talent/state.rs:169` no lee ni escribe campos de instancia: consulta
Core/NPC y usa un helper que devuelve `canonical_map_manager.is_some()`.
`SkillHandler.cpp:43–69` (`a5f8da2eb`) sitúa esa admisión en el handler de respec.
En F4 debe conservarse como helper privado del handler World, con el mismo
préstamo Core, constantes, ramas y resultado, al retirar el impl del estado;
no añadir esa responsabilidad al crate de instancias ni alterar la admisión.
Los siete bloques de instancia restantes contienen 42 métodos; los tres de
visibilidad contienen ocho. La preparación cerrada sobre `24c42d87d` incluye
ocho campos y cuatro defaults de fixture para Instances, y ocho campos y dos
defaults para Visibility. AdventureMapStartQuest conserva su DTO incondicional;
solo su campo de fixture está gated. El token de lock acompaña a binding, y el
plan de transportes de siete campos acompaña al publicador de visibilidad. Los
builders y coordinadores World permanecen en su lugar.
Anclas `a5f8da2eb`: `Player.cpp:19006,19016,19188,19195,20667`,
`Map.cpp:1853–1869,1929–1947,2878–2896`, `MiscHandler.cpp:890,910,968,1061`,
`AdventureMapHandler.cpp:24`, `Player.cpp:23337,25338–25381` y
`Object.cpp:3722–3729`. Se conservan en F6 las diferencias heredadas de
pending-bind en Session frente a Player, y el envío de transportes vacío que
Rust omite. La implementación local de ambos cortes está cerrada, con un único
dueño de las raíces y de los tres archivos de consumidores compartidos. Instances
traslada 41 métodos: el predicado de presencia de MapManager, usado únicamente
por respec, queda absorbido en la misma consulta Core del helper privado World.
No se conserva un método huérfano ni se altera su resultado. Visibility traslada
sus ocho métodos. Los 16 campos mantienen su orden de declaración y expresiones
de construcción mediante constructores explícitos; los seis defaults de fixture
también se conservan, con campos internos y accesos acotados para los consumidores.
El token de lock tiene un único proveedor en Instances y una fachada World para
su wrapper y tests originales. El plan de transportes y los tres helpers puros
de publicación tienen un único proveedor en Visibility; sus consumidores World
conservan las fachadas necesarias, sin acceso mutable al conjunto de transportes.
Se retira la fachada de fixture Visibility sin consumidores. No se traslada el
test del token, que usa Session y los stores de admisión.

La revisión por fuente compara los cuerpos trasladados y los helpers, y conserva
el clear/first-entry-wins de reset times, el take de pending bind antes del await
y las fences de generación/incarnation. En los 35 archivos de tests World
modificados se conservan los nombres y orden de sus 488 funciones (incluidas
las dos genéricas) y las 2.008
invocaciones de aserción; ese recuento léxico no ejecuta los casos ni prueba sus
resultados. Instances usa nueve dependencias internas y ninguna externa;
Visibility usa cuatro internas y tracing. El scanner reconoce las dos raíces,
sus mounts y procedencia, extendiendo los casos de dominio existentes sin retirar
las 424 regresiones anteriores ni regenerar baselines.
**NO VALIDADO:** no se ejecutó Cargo, formato, suites o inventarios para este
conjunto. No hay nueva evidencia de producción, paridad de respec, publicación
ni inicio de F5; queda pendiente la aceptación del macro completo.
El checkpoint local `c4febe9b2` guarda ambos dominios, sus consumidores y el
scanner en ese estado **NO VALIDADO**.

Preparación Loot sobre `24c42d87d`: el corte contiene 17 impls del estado y
76 métodos, incluidos los 26 de `session/loot/operations.rs`, con 18 campos
y sus expresiones originales de construcción. El cierre de los métodos puros
incluye `RepresentedLootRollState`, el evento de criterios de fixture, el
snapshot de criatura, `LootStoreRandomProperties`, la selección ponderada y
el builder compartido de `LootItemData`. Cada tipo/helper tendrá un proveedor;
los consumidores del shell conservarán fachadas donde sean necesarias. Las
autoridades, leases, scope/stamps y votos ya proceden de `wow-loot`, y los
comandos/identidad del roll de Core: no copiarlos ni introducir una arista World.
Hay dos solicitudes remotas async en el corte, master-loot y almacenamiento
del ganador; ambas conservan el canal acotado de una respuesta, `try_send`,
timeout de 250 ms, resultado de fallo y lifetime del claim. La identidad del
roll conserva su distinción respecto de GUID, allocation y generación.
Los workers de persistencia, reconciliadores y coordinadores ligados a Session
siguen en World para F5; el traslado de los impls no los arrastra por el nombre
Loot. No alterar la reserva/transferencia del claim, COMMIT desconocido,
quarantine, fanout ni orden de publicación. Anclas `a5f8da2eb`:
`LootHandler.cpp:216–290`, `Player.cpp:8723–8751`,
`Loot.cpp:277–399,454–510,575–614,695,758,915,995` e
`ItemEnchantmentMgr.cpp:153–212`. C++ sitúa Loot/Roll en sus objetos/Map/Player;
el cache y los rolls Rust en Session y sus rails async siguen siendo límites
heredados para F6, no nueva paridad demostrada. Esta preparación no implementa
Loot ni ejecuta aceptación. La preparación posterior confirma los 17 bloques y
76 métodos, contando también los siete genéricos: 26 en operations, tres en
random properties y diez en roll publication. El recuento de 73 del handoff se
corrige por lectura de las declaraciones; no se retira ningún método. Los tres
helpers adicionales no forman parte de esos 76. Los snapshots de GameObject,
su observación de instalación y `AuthoritativeLootReleaseLikeCpp` pertenecen a
los consumidores World/Cx que permanecen allí; no son tipos del impl trasladado.
Las dependencias directas preparadas son ocho internas
(constants/core/data/entities/loot/map/packet/world-core) y
flume/num-traits/rand/tokio; persistence y tracing quedan en los shells World.
Comienza el primer corte local de estado, proveedores compartidos y random,
con los dos tests independientes originales. Los demás impls y los accesos
directos de los consumidores se cerrarán en la misma entrega de Loot; ese primer
corte no acredita por sí solo la integración del dominio ni aceptación.
La fundación local de Loot está escrita: los 18 campos y su constructor
explícito conservan sus valores y orden; los campos son internos al crate y
los cuatro DTOs compartidos tienen un proveedor único. El bloque random
traslada sus tres métodos (dos genéricos), la selección ponderada privada y
el predicado de stacking con su fachada World. El builder de roll también
conserva una única definición y fachada para sus consumidores. La comparación
por fuente de los seis cuerpos y de los dos tests originales no encuentra
cambios de cuerpo tras normalizar namespaces/gates/espaciado; no ejecuta esos
tests. El siguiente corte está escrito: los seis bloques de authority, claims,
fanout, request-cache, rolls y publicación trasladan 25 métodos, incluidos tres
genéricos de publicación. La comparación con los cuerpos de `c4febe9b2` conserva
la operación; las diferencias revisadas son aliases de OwnedLootSnapshot/Scope
y del evento de criterios, formato de llamadas y gates de fixture emparejados.
Los wrappers World permanecen; el módulo de publicación de rolls solo contenía
el estado y queda retirado de World. Timeout 250 ms y criterio disenchant son
constantes privadas del dominio; los tres códigos de slot tienen proveedor único
y fachada World por sus consumidores reales. Se conservan las mismas dependencias.
El corte restante está escrito: diez bloques y 48 métodos, incluyendo money por
jugador, creature-dead y pass-on-group-loot. `session/loot/operations.rs` se
distribuye en hojas privadas de settings, views, autoridad, reconciliación y
transiciones de objeto. La comparación de los 76 cuerpos de Loot contra
`c4febe9b2` conserva la lógica tras los deltas revisados de rutas/gates/formato;
incluye los 26 cuerpos de operations, reconstruidos durante el movimiento, y
el alias Player del mismo `wow-entities`. Las 76 firmas conservan parámetros,
retornos, async y bounds genéricos; los aliases de Hub, MapKey, WorldCreature y
UnitDataValuesDeltaUpdate resuelven a los mismos proveedores inferiores. El
cambio de visibilidad permite los consumidores World del nuevo crate.
No se añaden dependencias ni se mueven
tests adicionales. Ya no quedan impls de LootState en World: permanecen los
accesos directos de consumidores y el scanner por cerrar. No se trasladan los coordinadores
durables ni los tipos de autoridad GO que pertenecen al shell.
El primer cierre de consumidores añade cinco APIs acotadas para el cache por
owner, el registro de una ventana aceptada y su first-open. `requests.rs` y
`request_cache.rs` usan esas APIs: se conservan lecturas por referencia, retiro
solo de loot_table, cuatro inserciones en el orden original y el clone de
authority en la cuarta. El try_send_packet sigue antes de los mirrors dentro
del callback de authority; ResponseEnqueueFailed conserva su descarte y cierre.
Los otros consumidores y fixtures todavía necesitan cerrar sus accesos.
Los Sources de Creature y autoridad GO también usan APIs por owner para
lectura/mutación de una entrada, insert/remove, generación y owner personal.
Sus 13 y siete cuerpos WorldSession conservan la fuente de `c4febe9b2` tras
expandir esas llamadas a los accesos originales. No se añaden clones ni se
exponen mapas mutables; los Cx y los accesos de Entities quedan pendientes de su
propio cierre.
Sources raíz y GO también cierran su cache, dinero personal y unique uses con
ocho APIs ganadas de Map/Set y retain por owner. La revisión de sus 14 y once
cuerpos WorldSession conserva lógica, clones, gates, retiros e inserciones; el
retain por pools mantiene el mismo short-circuit y las claves owner/player.
No se agrega un retiro por jugador sin consumidor ni se exponen maps mutables.
Release también cierra sus accesos con cinco consultas de vistas y las APIs de
cache existentes. Los cinco cuerpos World conservan su fuente tras expandir
esas llamadas: snapshot sin sort/fallback dentro del dominio, ordenado y
fallback en release-all, shares_storage/generación antes del claim y mismas
ramas de retiro/publicación. Los consumidores restantes siguen pendientes.
**NO VALIDADO:** estos cortes parciales no son un candidato compilado,
no acreditan integración del dominio ni sustituyen la aceptación del macro.

El censo léxico acotado de los impls restantes, sobre el mismo `24c42d87d`,
encuentra WorldEntities 19 bloques/70 métodos, Inventory 48/231 y Lifecycle
33/145. Son entradas de preparación, no una prueba de cierre de dependencias
ni de producción compilada. El único texto `WorldSession` en los bloques de
Inventory es una atribución C++ en un comentario de modifiers; no un parámetro
de sesión. Confirmar tipos, helpers, fixtures y consumidores antes de asignar
cada corte. Lifecycle arrastra obligaciones de finalización, callbacks de
rename, holders de pet load, puertos y fences: conservar su declaración/drop
order, admisión, cancelación y resultado de COMMIT. Las solicitudes y efectos
multidominio todavía ligados a Session conservan su dueño World para F5.
Las anclas iniciales localizadas en `a5f8da2eb` son
`Creature.cpp:333,353,696,2193`, `GameObject.cpp:2308,2501,3683,4053`,
`Player.cpp:9615,9882,11010,11069,11220,19312–19338`,
`WorldSession.cpp:162–188,544–709,908–949` y
`CharacterHandler.cpp:1520–1614`. No extender esa lectura a una afirmación
de paridad de los dominios completos ni modificar sus contratos por el censo.

El cierre preparado de WorldEntities confirma los 19 bloques/70 métodos y los
11 campos, nueve de producción y dos de fixture. Incluye los DTOs de spawn,
stats, kill, auras y uso/estado de GameObject con sus nominales inferiores
existentes, y los helpers de publicación de aura, filtro de candidatos e
inserción de criatura canónica. El tick legado y los flujos de loot/rewards
permanecen en World. `CreatureSpawnCatalogsLikeCpp` acompaña a la operación de
materialización: conserva los seis handles/rates, su Default fixture y una
fachada World para sus consumidores actuales; no se mueve el resto de las
capacidades de catálogo ni se crea una arista del dominio hacia World.
Los campos agregados del estado permanecen internos; los escritores World
existentes se cerrarán mediante accesos acotados, incluyendo el estado GO por
objeto. La fundación y el bloque de publicación de criatura están escritos:
cuatro métodos, el helper de aura y los dos tests puros originales tienen un
único proveedor en `wow-world-entities`, con fachadas World para los consumidores
reales. La comparación de fuente de los cuatro cuerpos conserva la lógica;
solo cambia la ruta de `VISIBILITY_RADIUS`, importada del mismo Core. Los
dos cuerpos de test y el helper conservan su fuente. El constructor mantiene
los once campos, expresiones y orden, usando imports de los mismos BTreeMap y
HashMap. `RepresentedGameObjectSpellCaster` tiene una sola definición en el
dominio y una fachada desde `instance_bind_contracts.rs`. Quedan 18 bloques y
66 métodos del estado, los otros helpers y los consumidores. Esta revisión de
fuente no ejecuta tests ni acredita aceptación. Las dependencias son diez
internas (constants/core/data/entities/map/packet/conditions/progression/loot/
world-core), sin nuevas externas ni dependencias de otros dominios World.
El resto de bloques, helpers y consumidores pertenece a la misma entrega,
todavía sin aceptación ejecutada. `Creature.cpp:688` es el comienzo de
`ApplyAllStaticFlags` y :696 el de `Creature::Update`. Se conservan también `GameObject.cpp:899,926` y
`Unit.cpp:10457` para los hooks de alta/baja y kill de esta preparación.

El siguiente corte Creature añade 26 métodos a los cuatro de publicación ya
trasladados. Los 30 cuerpos conservan la lógica de `c4febe9b2`, con imports del
mismo Core para WorldCreature, world_to_grid_coords y VISIBILITY_RADIUS. Los
DTOs de spawn materializado y snapshot de spell-click también tienen proveedor
en Entities y fachada World. Se corrigieron los tres exports cuyo proveedor
faltaba (filtro de candidatos, inserción canónica y su resultado), y el método
de fixture de despawn de vehículo también queda en Entities: catálogo y
snapshot ya son capacidades Core. Los helpers de amenaza, inserción, filtro y
conversión conservan sus cuerpos; los DTOs conservan campos y orden, con la
ruta explícita del mismo CreatureCreateData. Quedan los 40 métodos GO, los
consumidores directos y su aceptación; este corte no se declara integrado ni
validado.
El corte siguiente traslada nueve métodos de overrides GO y dos de query a
hojas privadas de Entities. Los once cuerpos conservan la fuente original,
incluidos los guards de owner, Some/zero, cambios canónicos y phase shift. El
método fixture de faction mantiene gate test/test-fixtures. Su llamada a
set_canonical_gameobject_spell_id queda cerrada por el corte canónico GO:
nueve métodos, incluido el mutador genérico, tienen cuerpos iguales a la fuente
original tras las rutas Core. Los coordinadores de tick/Update siguen World;
GameObject.h:227,239, GameObject.cpp:3683 y Object.cpp:2867 son las anclas
contrastadas de setters y amistad, sin nueva afirmación de paridad. Quedan 20
métodos GO y los consumidores.
Se trasladan otros once métodos de state/publicación y door/trap a dos hojas:
los cinco y seis cuerpos son iguales a la fuente original tras rutas/gates.
SetLootState, cooldowns, visibilidad y orden de transición se conservan; los
coordinadores World siguen allí. Las anclas exactas son UseDoorOrButton:2308,
ResetDoorOrButton:2296 y SetLootState:3683 de GameObject.cpp; destrucción usa
WorldObject::DestroyForNearbyPlayers en Object.cpp:3617, DestroyForPlayer:226 y
Player::HaveAtClient:23029. Quedan nueve métodos y el cierre de consumidores.
El último corte traslada esos nueve métodos: cuatro de fishing-hole en
gameobject_interaction y cinco de interacción/use en gameobject_use. Los nueve
cuerpos conservan la fuente de `c4febe9b2`; el tiempo de release se consulta
antes del lock, y AddUse, MaxOpens y SetLootState permanecen bajo el mismo lock.
Los 70 métodos originales del estado ya tienen proveedor Entities; no quedan
impls de ese estado en World, pero sus consumidores privados siguen pendientes.
La búsqueda usa el mismo CONTACT_DISTANCE de wow-movement, dependencia directa
ganada que eleva a once los edges internos; no se copia la constante. Las
anclas del último corte son LootHandler.cpp:270–312, GameObject.cpp:2501,3683.
Esta revisión de fuente no acredita compilación ni paridad ejecutada.

La revisión del helper de aura usa `AuraApplication::BuildUpdatePacket`
(`SpellAuras.cpp:229–289`) y `ClientUpdate` (:291–304), mismo `a5f8da2eb`;
la atribución heredada a `BuildUpdateData` no corresponde a una función de
ese target. El helper Rust conserva sus valores representados de aplicaciones,
duración y nivel, y recopila los puntos de efectos consecutivamente. C++ toma
stacks/charges y duración del Aura, distingue caster/item level y escribe los
puntos por índice de efecto, con estimated points cuando corresponda. Estas
diferencias preexistentes requieren el contrato y capturas de F6; no se corrigen
silenciosamente al trasladar el helper. La lectura de fanout identifica también
los gates efectivos C++ de `MessageDistDeliverer::Visit` y `SendPacket`
(`GridNotifiersImpl.h:38–105`, `GridNotifiers.h:182–191`); mover la selección
representada no acredita cobertura de shared vision, vehicle ni dynamic object.

El cierre preparado de Inventory conserva los 48 bloques/231 métodos. El
recuento anterior de 232 funciones incluía `visit`, helper anidado dentro de
`represented_inventory_descendants_postorder_like_cpp`; se conserva dentro
de esa operación, sin contarlo como otro método del estado. Sus
23 campos son uno de producción y 22 de fixture; el censo de 15 pertenece
al fixture anidado de PlayerItem, no a todo el estado. Los envíos que ya son
métodos del estado mediante Core forman parte de F4, conservando el mismo
publicador; el hecho de enviar un paquete no los convierte en un bloque
WorldSession/Cx de F5. El primer corte local cierra el estado, el fixture y
los DTOs de banco/gremio, auction, offhand y eventos de objetos mediante un
único proveedor y fachadas para sus consumidores actuales. Ese primer corte
está escrito, sin impls de operaciones trasladados: la comparación de fuente
conserva las 23 expresiones del constructor y su orden, y los 15 defaults del
fixture. El dominio empareja los gates de fixture con `test-fixtures` y propaga
la feature a Core; las fachadas World conservan sus gates originales. El
registro de raíces y la construcción mantienen el orden original de composición.
El siguiente corte está escrito: once bloques y 70 métodos de items, catálogo,
storage, storage-bags, storage-slots y persistence-load tienen módulos privados
en Inventory y sus bloques salen de World. La comparación de fuente con
`c4febe9b2` conserva los 70 cuerpos tras las adaptaciones revisadas de namespaces,
gates y formato; el `visit` anidado también permanece en su cuerpo. No se mueven
coordinadores, shims ni tests en este corte. Se añaden las dependencias reales
wow-data y num-traits; su registro global ya acompaña el corte.
La constante de armor penetration conserva 24 y corrige el ancla a `Unit.h:329`.
La revisión posterior encontró el consumidor productivo de
`handlers/character/stats.rs:446`, además del test World: el proveedor y export
Inventory quedan sin gate, y `session/combat/mod.rs` conserva solo la fachada
ungated. No quedan dos definiciones ni se modifica el cálculo. Siete métodos
llamados por estas operaciones aún
residen en otros archivos World; no se añaden stubs ni copias. Continúa su
cierre con equipamiento, modificadores y publicación. Quedan 37 bloques y 161
métodos del estado, los consumidores y el scanner en la misma entrega.
Todo este corte permanece **NO VALIDADO**. No trasladar los
coordinadores async de banco, carga o persistencia por compartir el archivo.

El siguiente corte Inventory añade dos bloques y diez métodos de enchantment,
con cuerpos iguales a `c4febe9b2` tras namespaces y gates de fixture. Los
wrappers, shims y aplicación coordinada de efectos permanecen en World. Los
helpers de stat changes, fields update y conversión de equipment set, junto a
LoadedEquippedItemEnchantmentsOutcome y append, tienen proveedor único del
dominio y fachadas World; sus nominales inferiores se conservan. Los mounts
solo declaran hojas existentes, sin stubs para las ocho futuras. El total
trasladado del estado es 13 bloques/80 métodos; quedan 35/151, con 18 bloques
en player_items y 17 fuera de esa carpeta, además del cierre de consumidores.
Equipamiento por slot y offhand añaden tres bloques/12 métodos con cuerpos
iguales a la fuente original. Los comentarios F6 se corrigen a las funciones
exactas, distinguiendo el caller de HasItemFitToSpellRequirements del helper
privado de slots. El escritor fixture World usa la operación de registro del
dominio. El corte posterior añade tres bloques/ocho métodos de equipment y
publication: CanUnequip, shapeshift, capabilities, getter fixture y los updates
de items conservan sus cuerpos; el builder usa la misma fachada del proveedor
Inventory. El total escrito del estado es 19 bloques/100 métodos; quedan 29/131
(12 en player_items y 17 fuera), los consumidores y la aceptación.

La lectura de `equipment_slots.rs` detecta un comentario heredado que excluye
ranged, aunque los métodos Rust resuelven `EQUIPMENT_SLOT_RANGED`. El target
`Player::GetWeaponForAttack` (`Player.cpp:9243–9273`) selecciona MAINHAND para
RANGED y además exige clase weapon y coincidencia de IsRangedWeapon. También
`Unit::UpdateDamagePctDoneMods` (`Unit.cpp:9033–9072`) multiplica offhand por
`GetTotalAuraModifier` (:4818–4844), que devuelve cero sin efectos; Rust
conserva la base 0,5 sin ese término. Son diferencias actuales a resolver con
el contrato de versión y la evidencia de F6. Corregir la atribución/comentario
al trasladar ese bloque, sin reparar silenciosamente slots, daño ni los tests
durante el refactor F4. Esta preparación no es aceptación ejecutada.

El contraste de `represented_has_item_fit_to_spell_requirements_like_cpp` usa
`Player::HasItemFitToSpellRequirements` (`Player.cpp:24641–24708`) y
`GetUseableItemByPos` (:9199–9209), mismo target. El selector de slot Rust consulta
objetos y requisitos sin el `CanUseAttackType` de C++. El contrato completo C++
tiene ignoreItem y la excepción de aura de shield no pasiva; el único consumidor
Rust localizado aplica pasivas cargadas durante login, de modo que esas dos
ausencias no demuestran por sí solas un fallo de ese consumidor. Se conserva la
operación actual en F4; su traslado no prueba la equivalencia completa del helper
ni autoriza ampliar callers o corregir gameplay sin el contrato de F6.

La preparación de apariencias conserva además el chequeo y los avisos del opcode
provisional de AccountTransmogUpdate. `CollectionMgr::SetAppearanceIsFavorite`
(:828) y SendFavoriteAppearances (:858) envían el packet, pero el mismo target
declara SMSG_ACCOUNT_TRANSMOG_UPDATE = 0xBADD (Opcodes.h:1894). Rust conserva su
alias provisional y evita emitirlo mediante el chequeo actual de appearance;
la fila/payload existe en wow-packet. F4 no inventa un ID ni acredita favoritos
en el cliente. Resolver ese contrato requiere evidencia de la versión en F6.

La preparación Lifecycle confirma 28 campos y las 28 expresiones originales
de construcción, incluidos cinco campos de fixture. El orden de declaración
conserva las obligaciones de drop registradas en `session/state.rs`: attachment
de battle pet, callbacks de rename, trackers durables, finalización, sender de
homebind y claim vivo. `src/finalization.rs` es el ledger de 15 pasos sin Session
ni SQL; su retención, resultado desconocido y prohibición de replay forman parte
del cierre del estado. `character_administration.rs` conserva la preparación de
rename sin escritura y la continuación de commit de un único uso; los callbacks
abortan reads al retirarse y retienen commits ya admitidos. El driver, la creación
de paquetes y su entrega siguen en World. Las cuatro familias de puertos en
`persistence_capabilities.rs` son capacidades tipadas, no autorización para
introducir un contexto universal ni campos públicos de traslado.

El tracker de money durable ya es de Core y debe reutilizarse. El tracker de
item durable, su guard, completion y fanout conservan el registro al drop, la
espera de idle y las gates de aplicación/publicación; moverlos requiere cerrar
todos los consumidores World sin duplicar estado. El holder de pet load mantiene
sus seis grupos de resultados tipados y el reset por personaje; no equivale al
Pet vivo. Las lecturas completas de `WorldSession::~WorldSession`, `LogoutPlayer`,
`LoadTutorialsData`/`SaveTutorialsData`, `HandleCharRenameOpcode`/`CallBack` y
`PetLoadQueryHolder`/su callback (`Pet.cpp:157–203,386–449`, mismo `a5f8da2eb`)
fijan los responsables C++ y el orden que debe contrastarse; no acreditan por sí
solas las rails async Rust ni paridad global. Esta preparación permanece en
lectura, sin implementación Lifecycle ni aceptación ejecutada.
Los seis tests del ledger de finalización y los tres de preparación/consumo de
rename son independientes de Session. El cuarto test de rename,
`production_session_driver_executes_ready_rename_callbacks`, usa el driver real
y permanece en World; comparte el port de fixture con los tres anteriores.
Cerrar ese fixture al trasladar los tests, sin borrar el caso integrado ni
duplicar los proveedores de la operación.
El censo 33/145 incluye los catorce métodos de puertos definidos fuera del
subárbol Session, en `src/session_persistence_capabilities.rs`. Sus cinco
constructores de capacidades (`required_like_cpp`) también acompañan a los
tipos para evitar impls de tipos externos en World; la composición World y
sus setters que delegan permanecen allí. Mantener los setters/getters tipados
existentes y cerrar sus consumidores sin publicar los campos del agregado.
El tracker de item tiene además un test puro de la transición watch entre
la observación busy y el primer poll: conserva ese test y su fuente al mover
el tracker, sin atribuirle evidencia SQL ni validación ejecutada.
Los loaders del estado en `handlers/spell/ops_1.rs` y `ops_2.rs` usan además
`WrappedGiftRow`/`WrappedGiftLoad` y el contrato representado
`LootTemplateRow`/`LootTemplateTable` de `handlers/spell/state.rs`. Deben cerrar
esos DTOs y los dos métodos de Table con el proveedor de la operación cargadora,
manteniendo fachadas para los handlers World. El `LootTemplateRow` inferior de
`wow-loot` tiene otra estructura (entry más LootStoreItem); no es un alias
intercambiable con esta fila cargada de nueve campos. Los helpers y condiciones
que ya pertenecen a `wow-loot` se reutilizan. No crear una copia, convertir el
loader en otro owner ni alterar el orden de sus awaits para resolver el import.

El primer corte Lifecycle está escrito: el ledger completo y sus seis tests
originales viven en `wow-world-lifecycle`, con fachada desde `src/finalization.rs`.
La comparación completa de fuente contra `c4febe9b2` solo cambia pub(crate) a pub
para las llamadas entre crates y la ruta/import de tests. Mantiene los campos
privados, orden de steps, máscaras, InFlight antes del poll, retención y ausencia
de replay/reset. Ese corte gana wow-map y wow-persistence, publish=false y
registro global, sin ejecutar los seis tests. La preparación/commit de rename
también está escrita, con sus tres tests puros y un fixture compartido de campos
privados. La comparación de la operación contra `c4febe9b2` conserva su fuente
salvo visibilidad; los cuatro cuerpos de test conservan lógica y aserciones tras
cambiar el acceso al fixture por commit_snapshot/commit_count. El caso del driver
real permanece World. Tokio se habilita solo para tests y test-fixtures en este
corte; su delta de lock/política aún debe cerrarse. Los 28 campos
y los 33 bloques/145 métodos de Lifecycle aún no se trasladan en este corte.
**NO VALIDADO:** no hay aceptación, paridad de logout ni evidencia DB/live nueva.

Los callbacks de rename también tienen proveedor Lifecycle: Read/Drop, Commit,
Delivery, ready y RenameCallbacks. Ocho cuerpos y el orden/tipos de campos
conservan exactamente la fuente de `c4febe9b2`. Sus colas y handles permanecen
privados; World usa conteos, un resultado por índice, registro de entrega y
polling de la tanda. La revisión del coordinador conserva el procesamiento de
commits anteriores, ausencia de publicación de commits nuevos en la misma
pasada, registro de toda la tanda en FIFO, limpieza antes de kick y retención
de handles durante cancelación de finish. Paquetes, logging, fase del driver y
admisión siguen en World. Se ganan wow-core, flume, Tokio normal y tracing por
los usos trasladados; lock/política están pendientes. Los otros tipos necesarios
para mover SessionLifecycleState todavía pertenecen a World.

El tracker de item loot y su cierre de tipos también tienen proveedor
Lifecycle: fanout retenido, completion, estado privado, tracker, guard y Drop.
La revisión completa de fuente conserva tipos/orden de campos, subscribe antes
del busy check, registro de completion antes del decremento, unlock antes de
wake, wrapping/saturating y el único test de la carrera de watch. El delta
revisado es visibilidad, import del mismo OnceLock y nombre del módulo de
tests. World conserva las fachadas y la publicación/SQL; el estado interno no
se exporta. Se reutilizan PlayerRegistry de Core y authority/snapshot de
wow-loot; esos dos edges ganados aún requieren cerrar lock/política. El test no
se ejecutó y no hay evidencia de durabilidad nueva.

Tres contratos de valor adicionales viven en Lifecycle: AccountData, el job
Homebind y el evento fixture de RemoveAtLoginFlag. Mantienen campos/orden y
derives; el job no gana Clone. Las tres máscaras y default_account_data también
tienen proveedor único, usando NUM_ACCOUNT_DATA_TYPES de wow-packet sin copiar
15. World conserva fachadas/gates y sus funciones de sesión. La revisión usa
WorldSession.h:826–857 y WorldSession.cpp:834 para enum, masks, default y load;
no acredita DB/publicación. El nuevo edge normal wow-packet acompaña el contrato.
El lock y la política ya reflejan los seis edges internos de Lifecycle
(core/loot/map/packet/persistence/world-core) y flume/Tokio/tracing normales,
sin edges build. Su test-fixtures permanece vacío. Es integración de fuente,
sin ejecución de Cargo ni checker.

El holder de pet load y sus seis tipos CharacterPet*RowLikeCpp tienen proveedor
único Lifecycle. Conservan derives, campos, orden y ObjectGuid; no se sustituyen
por filas binarias del port de persistencia. Sus seis mapas son privados y la
API acotada conserva get por referencia, insert/remove y sus resultados, clear
de spells y reset por personaje. Los cuatro consumidores World conservan los
filtros, conteos, clones y orden de invalidación/materialización originales tras
expandir esas llamadas. El Pet vivo y los loaders/SQL siguen en World, con las
fachadas nominales originales; no se añaden dependencias ni evidencia DB/live.

**Checkpoint local F4b — 2026-10-03, NO VALIDADO:** este lote guarda los nuevos
roots Loot, Entities, Inventory y Lifecycle y sus cierres de fuente revisados
sobre `c4febe9b2`. Loot conserva los 76 métodos originales y Entities los 70;
Inventory lleva 100 de 231. Lifecycle aún conserva sus 28 campos y 145 métodos
en World, con los contratos auxiliares trasladados descritos arriba. Faltan los
consumidores/fixtures privados, el resto de Inventory, la extracción del estado
Lifecycle y F5–F6. No se ejecutó Cargo, formato, pruebas ni aceptación para este
lote; no es un candidato compilado y no cambia el gate rojo R1/publicación de
P4b ni su evidencia histórica. El trabajo autorizado continúa en la misma rama.
El checkpoint guardado es `b7a6b7a43` (176 archivos, árbol limpio al commit).
La continuación de fuente añade las consultas de rolls por clave y snapshots
con la misma iteración/copia, y cierra rolls/fanout World. El driver de Entities
usa cinco operaciones acotadas de tick y take: conserva las lecturas separadas
4/2/4 y completa la cola de loot antes de extraer rewards. Las cinco familias de
capacidades de persistencia y sus required constructors tienen proveedor
Lifecycle, con campos internos y mismos tipos/orden/derives; los 14 métodos del
estado aún esperan su extracción. Los dos bloques de equipment sets añaden 14
cuerpos iguales a la fuente original, conservando su helper único y el
coordinador World; Inventory lleva 21 bloques/114 métodos, con 27/117 pendientes.
Estos cambios posteriores no forman parte del SHA guardado ni tienen aceptación.
La fundación posterior de Lifecycle también está escrita: 28 campos y sus 28
expresiones iniciales conservan tipos, valores y orden de declaración/construcción
tras los aliases y gates revisados. Los cinco campos de fixture, inicializadores
e imports se emparejan con test-fixtures en el dominio y conservan los gates World.
El fixture de loaded flags conserva tres campos y su Default; el intervalo de
guardado tiene proveedor único Lifecycle y fachada World. Los campos permanecen
internos y los 145 métodos/consumidores se cierran en la misma entrega, sin pruebas
ejecutadas ni nueva afirmación de persistencia/paridad.

La continuación revisada de Inventory añade appearance (19 métodos), valuation
(6) y durability (5), después de equipment sets: 25 bloques/144 métodos escritos,
con 23/87 todavía en World. Los cuerpos conservan la fuente original tras aliases
y gates; durability usa el mismo `wow_entities::INVENTORY_DEFAULT_SIZE`. Los
coordinadores de pérdida, reparación, transacciones y publicación siguen en World.
El contraste de reparación retiene un límite previo para F6: en `a5f8da2eb`,
`Player::DurabilityRepairAll` (`Player.cpp:4629–4711`) usa
`GetInventorySlotCount()` para el backpack; el helper Rust de costes conserva
`INVENTORY_DEFAULT_SIZE`. Ambos coordinadores Rust de reparación con dinero
consumen esa lista. El traslado no corrige ni acredita ese conjunto para un
backpack ampliado; F6 debe cerrar la operación completa y su publicación.
Lifecycle añade los 16 métodos de puertos/identidad, dos de login claims y seis de
finalización, cleanup y logout: 6 bloques/24 métodos originales trasladados,
con 27/121 pendientes. Conserva la reserva Weak por GUID, los tokens de retiro y
la admisión de finalización; el coordinador async permanece en World.
Los cierres de consumidores Loot en authority/claims/money/persistence y los tres
handlers conservan snapshots, fallback, orden de claims/durabilidad y publicación.
Entities cierra tracking de aura y las hojas de uso básico, tipos y PvP mediante
operaciones por GUID, manteniendo los clocks y eventos originales. Los demás
consumidores privados y fixtures siguen pendientes. Estos cambios posteriores a
`b7a6b7a43` son fuente revisada y **NO VALIDADA**; no se ejecutó Cargo, formato,
pruebas ni aceptación y no modifican R1 ni el resultado del presupuesto histórico.

El corte posterior de bank mueve dos bloques/16 métodos a Inventory: 27/160
escritos y 21/71 pendientes. La revisión conserva los 16 cuerpos tras gates y
el mismo bridge Core para los updates; World conserva reparación y transacciones.
Lifecycle mueve los seis métodos de filas de colección, temporizador y CUF:
7/30 trasladados y 26/115 pendientes. Los tres DTO de filas conservan diez campos,
derives y orden con proveedor único Lifecycle. Su feature de fixtures se reenvía
a Core y la dependencia de desarrollo activa el mismo gate del Hub.
Los consumidores GameObject de scripts, tick y delete usan consultas por GUID e
iteración de solo lectura del mismo BTreeMap; las cuatro retiradas de cache Loot
siguen en sus puntos originales. El registro de jugadores, kill y trap despawn
reutilizan las consultas existentes de Loot. Inventario/bolsas conservan el
cortocircuito, la consulta sin snapshots y el retorno temprano; los hooks de item
storage conservan el orden de clonación de Arc bajo el mismo gate World. Todo
permanece **NO VALIDADO** y los cierres de consumidores continúan.

**Checkpoint de continuación local F4b — 2026-10-03, NO VALIDADO:** Inventory
añade dos bloques/19 métodos de modifiers, alcanzando 29/179 originales
trasladados y 19/52 pendientes. La comparación conserva sus 19 cuerpos tras
gates; cuatro operaciones de fixture sustituyen cinco accesos World, con los
mismos append/clones y orden de aplicación. Los slots usan su proveedor Entities;
`reputation_to_rank_like_cpp` conserva Progression y gana su edge normal real,
registrado también en lock y policy. Lifecycle alcanza 8/36 métodos originales
trasladados, con 25/109 pendientes, tras los seis métodos de cuenta. Conserva
borrow/Arc, snapshots, retención antes del await y clasificación de persistencia.
El temporizador tiene setter/consultas acotados; los coordinadores y las pruebas
World conservan sus condiciones y aserciones. Los once append de eventos de muerte
y el mutador de trap mantienen sus gates/orden mediante APIs de Entities. Los dos
setters de hooks de loot conservan movimiento de Arc y el gate World original.
Este checkpoint guarda fuente revisada y los cierres de consumidores anteriores;
no ejecuta compilación, formato, tests, scanners ni aceptación, no acredita paridad
nueva y no habilita publicación. Continúan los métodos restantes, consumidores,
fixtures y F5–F6 dentro de la misma entrega autorizada.

El checkpoint de continuación guardado es `d7b0d72191494e063d9ffe7b967c3682b4a84e42`
(81 archivos, árbol limpio al commit). La unidad posterior de Inventory mueve
dos bloques/cuatro métodos de publicación de nuevos items, preflight de swap y
duración: 31/183 originales trasladados y 17/48 pendientes. La comparación de
fuente conserva los cuatro cuerpos, sus providers y el orden de almacenamiento
canónico y publicación. Los wrappers y las transacciones permanecen en World.
Esta unidad continúa **NO VALIDADA** y no modifica la evidencia del checkpoint.

Lifecycle añade el método genérico de resultado de transacción de dinero:
9/37 originales trasladados y 24/108 pendientes. Los contratos de exclusión,
cancelación y reconciliación tienen proveedor Lifecycle y fachadas World;
la comparación conserva los cuerpos, la clasificación de COMMIT desconocido,
el armado antes del await y el orden de campos save-fence/mutex. El constructor
reemplaza el único literal World en el mismo punto y sin nuevas adquisiciones.
Las consultas Loot de items, tappers y encounter, la limpieza de cache y la
consulta del GUID activo conservan operación, préstamo y gates; gameobjects
consulta fases por GUID y el mismo sufijo de efectos de pesca, sin nuevo snapshot.
Estos cambios son inspección de fuente **NO VALIDADA**, sin evidencia nueva de
durabilidad, protocolo, aceptación ni publicación.

Inventory añade buyback y void storage (dos bloques/20 métodos): 33/203
originales trasladados y 15/28 pendientes. La comparación conserva todos los
cuerpos tras gates, incluidos canónico primero, selección de buyback y snapshots
de void storage; los wrappers y tests World siguen en su dueño. Lifecycle mueve
otros 19 métodos de puertos del bloque de planes: 56/145 métodos originales
escritos y 89 pendientes; el bloque World conserva sus seis métodos restantes.
Se mantiene la distinción entre préstamos y clones de Arc. Este avance continúa
**NO VALIDADO**, sin ejecutar compilación, suites, formato ni aceptación.

La continuación de dinero, monedas y escalado de Inventory mueve cinco
bloques/nueve métodos: 38/212 originales trasladados y 10/19 pendientes. Los
nueve cuerpos conservan la fuente tras los gates de fixtures, incluidos el
retorno de autoridad canónica, el fallback sin handle y los límites de nivel.
La lectura de fase, sus dos retiradas y el cutoff de movimiento de login usan
operaciones acotadas de Entities en los mismos puntos; el setter recibe el
instante calculado por el caller. Continúan los consumidores y fixtures; este
avance sigue **NO VALIDADO** y no reemplaza la campaña final.

Lifecycle añade los 24 métodos de cuentas, colecciones, battle-pet/login y
despacho de módulos: 80/145 originales trasladados y 65 pendientes. La comparación
conserva los cuerpos tras aliases y gates, incluido el retorno de autoridad de
slots completa y el descarte de un lote de efectos inválido. Las dependencias
directas de constantes, Entities y Module API se ganan por sus consumidores
existentes; no activan una integración nueva. Se conserva la limitación previa
del update de heirlooms con opcode pendiente, sin reparación de protocolo.
El contrato de delta de moneda (siete campos/derives) y su helper de máximo tienen
un proveedor Core; la fachada World reutiliza además la tabla de equipos por
raza ya existente en Core, con los mismos valores y fallback.
Las fixtures de cache Loot adaptan 213 accesos en 34 archivos mediante consultas
por clave y un iterador prestado. La comparación de fuente conserva el orden de
409 nombres de función y 1.802 macros de aserción; no se ejecutaron esos tests.
Continúan los demás campos de fixture y los consumidores privados. Este corte
permanece **NO VALIDADO** y no acredita paridad ni durabilidad nueva.

El corte de transferencia y preparación mueve cinco bloques/nueve métodos de
Lifecycle: 89/145 originales trasladados y 56 pendientes. Conserva destinos,
normalización de orientación, fases post-add, recuperación terminal, fences de
save diferido y selección de residencia; el enum de save conserva sus cinco
variantes y deriva con proveedor único Lifecycle. World conserva coordinadores,
ACK de proyección y tests. Inventory mueve cuatro bloques/seis métodos de
publicación, carga y consulta: 42/218 originales trasladados y 6/13 pendientes.
Los cuerpos y el retorno de envío se conservan; el flag resting conserva `0x20`
con proveedor Core junto a los demás flags del Player y fachada World.
Entities cierra 43 accesos privados de producción en 15 archivos con las APIs
existentes; mantiene consulta, modificación condicional, inserción si falta y
append/lectura de eventos en sus puntos. Continúan las fixtures. Es revisión de
fuente **NO VALIDADA**, sin atribuir al traslado aceptación ni paridad nueva.

Inventory completa la escritura de sus 48 bloques/231 métodos originales con
los 13 de void storage, turn-ins y estadísticas. Los cuerpos conservan valores,
orden y providers tras aliases; los dos DTO de void storage, su Default y el enum
de turn-in tienen proveedor único Inventory y fachadas World. La dependencia
normal de Persistence se gana por el tipo SQLx-free de merged item write. La
constante de hit melee conserva `5`, con proveedor Inventory y fachada de combat.
Esto completa la escritura de esos métodos, no los consumidores ni su aceptación.
Lifecycle alcanza 95/145 métodos escritos y 50 pendientes tras seis de planes;
el error de dinero conserva seis variantes y sus cuerpos Display/Error, y el plan
de talento conserva tres campos/derives. Sus consumidores determinan la visibilidad
de quarantine y persist gold. Loot adapta otros 97 accesos en 20 archivos de
fixtures de handlers y el lector bootstrap; sus setters de fixture conservan
operaciones literales y reutilizan las inserciones de propietario/dinero existentes.
Las fixtures de sesión de Entities adaptan 295 accesos en 21 archivos mediante
seis operaciones de fixture; el loop de aura evalúa el reloj por cada elemento.
El conjunto modificado de 27 fixtures de sesión, que incluye el cierre de cache
Loot anterior, conserva 315 nombres de función y 1.330 macros de aserción en la
comparación de fuente. Todo continúa **NO VALIDADO**, sin ejecutar esas suites,
compilación, formato, scanners o aceptación; consumidores, fixtures y F5–F6 siguen
dentro de la entrega autorizada.

El siguiente corte de carga traslada los dos bloques/24 métodos de Lifecycle a
`state/load.rs`: 119/145 originales escritos y 26 pendientes. La comparación de
fuente conserva los 24 cuerpos tras el gate de fixtures y la ruta del único helper
de bonus de heirloom, trasladado desde `collection_adapter`. World conserva sus
coordinadores y fachadas; Data y Progression son dependencias directas por el
catálogo y la fila de reputación actuales. El import del snapshot de powers queda
bajo el gate de su uso. Las anclas contrastadas de `a5f8da2eb` son
`CollectionMgr.cpp:113,174,222` y `WorldSession.cpp:834,908`; no prueban paridad
de todos los loaders. Se conservan para F6 los contratos previos de catálogo
ausente y repetición de toys: el C++ usa `emplace`, mientras la carga Rust existente
usa `insert` sobre el mapa nuevo; no se repara dentro del traslado.
Los consumidores de playtime adaptan 13 accesos en siete archivos mediante seis
consultas/asignaciones escalares. El reloj, elapsed y las sumas saturadas conservan
sus puntos World. Entities adapta otros 51 accesos en 12 fixtures de handlers con
las APIs por GUID y slices prestados; la comparación de fuente conserva 201 nombres
de función y 571 macros de aserción en esos archivos. Estas comparaciones son
inspección de fuente, **NO VALIDADA**, sin ejecutar compilación ni tests.
El cierre acotado de consumidores Inventory adapta otros 28 accesos en 12 archivos
de producción, incluidos accesos gated de fixtures. Conserva arrays/slices prestados,
el clon único de equipment sets y el append de eventos; la llamada de enchantment
reutiliza el append de una sola acción con un clon, como el push anterior. Queda el
acceso de carga de void storage, reservado por solapamiento, y 78 accesos cualificados
en 16 archivos unitarios de sesión, según la lectura actual de campos declarados.
Es un censo léxico acotado, no prueba de compilación ni cierre de aliases.
Lock y política reflejan las seis aristas normales ganadas: Inventory → Persistence;
Lifecycle → Constants, Data, Entities, Module API y Progression. No se modifican
ceilings, baseline ni gates de aceptación/publicación.
La revisión detectó 17 APIs fixture todavía ausentes tras adaptar esos consumidores;
se completan en el módulo privado gated `fixtures.rs`. La lectura posterior encuentra
un proveedor único para cada nueva llamada, con los mismos push/extend y préstamos.
Esto cierra ese hallazgo de fuente, sin sustituir compilación ni aceptación.
El corte queda guardado en `9e001cc1b9d2c0c417d7269285435df3676eeb10`, con 177
archivos y árbol limpio al commit de las 05:34:01 UTC. Es checkpoint local
explícitamente **NO VALIDADO**; R1, la campaña ordinaria de 600 s, aceptación del
macro y gates F5–F6 conservan su estado pendiente.
El acceso de carga de void storage se cierra después con una inserción fixture
concreta por slot y item prestado. Conserva guard de owner, indexación, rechazo de
slot ocupado/id repetido y un único clon; `AddItemAppearance` sigue después del
éxito. `Player.cpp:18334–18373` de `a5f8da2eb` contiene `_LoadVoidStorage`: el
C++ asigna el slot tras sus guards, mientras el fallback Rust previo rechaza
ocupación/duplicado; esa diferencia se conserva y necesita contrato/evidencia F6.
Runtime traslada otros cinco métodos Lifecycle (124/145 escritos, 21 pendientes)
y las dos constantes literales de AntiDOS a su consumidor único. Los cuerpos
conservan canales, callback, paquete tutorial, reloj y requeue según resultado;
el enum de fase usa su proveedor Core. Se corrigen las anclas del timer a
`WorldSession.cpp:505–511`, manteniendo el orden de packet loop/callbacks/decisión.
Este corte sigue **NO VALIDADO** y no acredita equivalencia de COMMIT/cancelación
con el C++ síncrono ni aceptación de lifecycle live.
El corte de commit traslada otros dos métodos a `state/money_persistence.rs`:
126/145 originales escritos y 19 pendientes. Conserva guard, clone del puerto,
transacción, await y retorno del guard de exclusión; el shortcut y tombstones
mantienen su gate de fixtures. El símbolo target localizado es
`WorldSession::HandleTrainerBuySpellOpcode` en `NPCHandler.cpp:132`, no el nombre
`SpellHandler::HandleTrainerBuySpell` de la asignación inicial. El traslado de
fuente no demuestra por sí mismo la equivalencia completa de compra/COMMIT.
Inventory cierra los 78 accesos cualificados de 16 fixtures de sesión mediante
16 operaciones gated por clave, slot, valor o slice. Conserva los préstamos de
item/currency, el retorno de insert y el acceso `HashMap[index]`; no añade clones
ni mapas mutables públicos. La comparación de fuente mantiene 208 nombres de
función y 1.024 macros de aserción en esos archivos. La lectura actual de campos
declarados no encuentra accesos `.inventory.<campo privado>` en World src ni
unit_tests; es cierre léxico acotado, no prueba compilada de todos los aliases.
Tutorials cierra 18 accesos en cuatro archivos World y añade nueve operaciones
acotadas entre `state/account.rs` y `state/persistence_ports.rs`. El reset conserva
sus cuatro asignaciones antes del puerto/await; set-int conserva índice y dirty
solo por cambio. Proyección/ACK permanecen World con snapshot, grupos, guards y
fences; solo limpian dirty cuando el array actual coincide con el confirmado.
El puerto mantiene una sola copia de Arc. Todo sigue **NO VALIDADO**, sin ejecutar
Cargo, suites ni aceptación, y los restantes consumidores/fixtures Lifecycle
continúan dentro del macro autorizado.
La carga auxiliar traslada seis métodos a `state/login_load.rs`: achievements y
su fixture, toys/heirlooms, glyphs y action buttons. Los seis cuerpos conservan
la fuente original, incluidos vaciado, resultado inesperado, loaded flags y
orden de carga; la fixture conserva test/test-fixtures. Group traslada un método
y su converter de once variantes a `state/group.rs`; la dificultad de instancia
y tests existentes usan la fachada World del mismo proveedor. Se gana la arista
normal Lifecycle → `wow-social`, crate inferior, por el intent tipado actual.
La metadata de esa arista se incorpora manualmente a Cargo.lock y a la allowlist
de dependencias, sin añadir paquetes, cambiar versiones ni ampliar techos. Las
dos pruebas puras del converter Group conservan cuerpos y aserciones en
`wow-world-lifecycle/unit_tests/group_mapping.rs`, montadas con `cfg(test)`.
La carga de cadáveres mueve un método a `state/corpses.rs` y conserva la
comprobación bajo lock, su liberación antes del await del puerto y la segunda
adquisición para materializar. Core mantiene un único proveedor de los dos DTOs
y los dos helpers, con fachada nombrada World; conserva el consumo del GUID antes
de rechazar coordenadas y la marca loaded después del bucle. La prueba pura
conserva su cuerpo en `wow-world-core/unit_tests/map_manager_tests/corpse_load.rs`.
Los anchors consultados son `Map::LoadCorpseData` (`Map.cpp:3623–3705`) y
`Corpse::LoadCorpseFromDB` (`Corpse.cpp:181–234`); esto no demuestra equivalencia
async por sí solo. El bootstrap traslada sus dos métodos a `state/bootstrap.rs`
con el gate de fixtures explícito y conserva sus cuerpos. Core mantiene los
proveedores únicos del selector de poder y de `default_display_id`; World
conserva solo la fachada requerida para el segundo. Sus veinte combinaciones y
fallback 49 siguen iguales, sin afirmar que coincidan con los datos `ChrRaces`.
Los veinte accesos al pet-load holder en cuatro consumidores World pasan por
operaciones tipadas por familia/número, conservando préstamos y los clones ya
existentes. El holder conserva sus seis mapas privados. Los ocho accesos del
coordinador de rename pasan por forwards tipados, manteniendo los resultados
prestados, conteos, admisión, enqueue/poll y drain de sus handles; la entrega
y los kicks permanecen en el coordinador World.
Otros once accesos a puertos y tres al tracker de objetos se cierran mediante
getters/setters tipados y delegates de begin/wait/take. Las fachadas conservan
una clonación por getter y los guards/await originales. Diez accesos del flujo
de conexión/logout usan las operaciones existentes y un setter literal de
loading: limpiar timer antes del ACK, loading antes de liberar claim y loading
antes de sincronizar visibilidad. La fábrica lazy y el único worker FIFO de
homebind pasan a `state/homebind.rs`; conserva recepción, await, clasificación,
warnings y envío, con el mismo `SendError<Job>`. World mantiene guards y request
antes de llamar a esa cola (`Player::SetHomebind`, `Player.cpp:17023–17038`).
Las quince fixtures de Lifecycle migran 74 accesos directos mediante doce APIs
acotadas de fixture y getters normales. Las seis consultas de vacío examinan
los mapas completos del holder; no sustituyen vacío global por ausencia de una
clave ni exponen mapas. Account-data conserva el préstamo por índice, los
timestamps crudos y el retiro específico del puerto. La comparación de fuente
conserva 196 nombres de función y 808 macros de aserción en el mismo orden.
Esto no acredita compilación, ejecución ni paridad de esas pruebas.
Los nueve métodos restantes de persistencia de spell se trasladan a
`state/stored_item_loot.rs`. Sus cuerpos y los de las dos pruebas puras de
helpers conservan la fuente original; las pruebas se montan fuera de `src`.
Cuatro DTOs salen mediante exports nombrados en State y el root, manteniendo
privado el módulo de contratos; los dos helpers quedan internos al crate.
World conserva las fachadas con consumidores reales y retira los imports
que quedaron sin uso. Lifecycle alcanza **145/145 originales escritos**.
Los diecisiete accesos restantes a flags, at-login y customizations se cierran
con operaciones crudas gated: el reset mantiene None/None/false, el registro
mantiene un push y el lector mantiene un slice prestado. El módulo fixture
queda en veinte APIs ganadas; se retiran dos propuestas que no tenían consumidor.
La lectura de campos declarados no encuentra accesos cualificados
`.lifecycle.<campo privado>` en World src ni unit_tests. No demuestra el cierre
de todos los aliases ni la resolución compilada de providers y montajes.
Sigue **NO VALIDADO**, sin ejecución de Cargo/suites ni cierre del macro.
La revisión acotada de exports nombrados de Inventory y Lifecycle encuentra
providers únicos, visibilidad y gates compatibles por fuente; no equivale a
resolución compilada. La preparación de tests independientes de los nueve
dominios conserva los cinco cuerpos puros trasladados (grupo: dos, corpse: uno,
stored-item loot: dos). No identifica otros cuerpos World trasladables intactos:
las pruebas de sesión, mapa, ownership y transporte permanecen en sus seams.
Interaction, Loot, Entities y Spell ya contienen familias puras locales; no se
añaden pruebas espejo de getters para aparentar cobertura. Falta contrastar en
aceptación el conjunto ejecutado, sus gates y la composición de producción.
La revisión de los exports Inventory detecta un provider perdido durante la
extracción: `item_storage_fields_values_update_like_cpp` tenía fachada y callers,
pero ninguna definición. Se recupera en `publication.rs` desde `b7a6b7a43^`, con
cuerpo literal idéntico, incluidos bits/máscara y el único clone original.
World conserva su fachada y la regresión de banco/enchant integrada; no se
modifican bytes ni se acredita que esa regresión haya vuelto a ejecutarse.

El scanner ya incorpora los roots y roles de Loot, Entities, Inventory y
Lifecycle. Se extienden las pruebas existentes de montaje y de rechazo de
SessionCore/WorldSession en dominios; no se añaden ni renombran tests, ni se
regenera el baseline. El examen del diff conserva los gates de autoridad
anteriores. Esta adaptación no se ha ejecutado y no acredita inventario ni
aceptación de los nuevos roots.

### F5 — handlers y orquestación

**Checkpoint de fuente anterior al diseño — 2026-10-03:**
`062fc7803e0fb2932d2ea02623933096d669147c`, árbol limpio al guardarlo
a las 06:20 UTC; 124 archivos, 3.030 líneas añadidas y 2.141 retiradas,
incluido el rename de la prueba de corpse. Guarda el resto de los métodos
originales de Lifecycle y sus consumidores/fixtures, los cinco tests puros
trasladados y el provider Inventory recuperado. **NO VALIDADO**: no se ha
ejecutado Cargo, formato, suites, scanners ni aceptación para este lote.
No acredita F4 terminado, paridad, publicación ni cumplimiento de R1/600 s.

Diseñar con el censo actualizado después de F4:

- Fijar el contrato de registro sin ciclos, el lugar de definición de los contextos y sus
  builders, la composición y la adaptación de futures/lifetimes.
- Mantener el conjunto exacto de opcodes, metadata, admisión, conexión y orden observables;
  incluir las pruebas de registro y composición de producción correspondientes.
- Handlers en sus dominios mediante contextos acotados; orquestación multidominio
  (login, loot, quest rewards) en responsabilidades de aplicación por encima de los dominios.
  El shell conserva driver y composición, sin convertirse en otro servicio universal.
- Desbloquear `player_quest_gameplay_snapshot_like_cpp`,
  `mutate_player_quest_gameplay_like_cpp`, `resolved_group_guid_like_cpp`,
  `mutate_player_spell_runtime_like_cpp`, `sync_player_registry_state_like_cpp` y los
  accesores de trade. Acotar `represented_owned_loot_authority_like_cpp` y `as_context(self)`
  en commits dedicados, preservando comportamiento.
- Revisar los 202 split thunks con más de dos sitios y los 61 diferidos por legibilidad.

Objetivo físico: `wow-world` con 20k–40k líneas de producción. Es un criterio junto con los
dueños semánticos y la migración completa de consumidores; el número por sí solo no demuestra
modularidad ni paridad.

#### Propuesta concreta F5 para revisión — 2026-10-03

Base de diseño: `062fc7803`, extracción F4b escrita pero no aceptada. La lectura
actual de `handlers` encuentra 397 literales `PacketHandlerEntry` y 397
`inventory::submit!` en 38 archivos. Seis son templates de macros: las 87
invocaciones en client_state, movement y chat/channels dan
`397 - 6 + 87 = 478` entradas por fuente, coherentes con las 478 filas de
ambos snapshots actuales. Son 19 null handlers, 28 movement, 16 movement ACK,
10 speed ACK, cinco channel commands y nueve channel-player commands.
El conteo literal de submits es también 397 en el SHA aceptado `3ea514530`;
no demuestra por sí solo el conjunto enlazado ni reemplaza su test exacto.
El conteo físico acotado de `wow-world/src`
da 509 archivos Rust / 141.635 líneas, incluidos comentarios, blancos y ramas
cfg; **no es R5 ni LOC de producción compilada**. Los antiguos 2.598 métodos,
443 thunks y 202/61 cortes no son un censo vigente: deben reconciliarse con
los owners y consumers actuales durante F5 y con los instrumentos existentes
en la aceptación final. Ninguna de estas lecturas ejecuta una validación.

Se propone separar el tipo del registro de su receiver concreto, conservar
thunks monomórficos y construir los contextos mediante préstamos disjuntos.
La alternativa de mover la entrada actual sin cambiar su tipo crea el ciclo
`wow-handler -> wow-world -> wow-handler`. Mantener todos los registros y
adaptadores en World conserva el acoplamiento que F5 debe retirar. Un registro
de closures capturadas añade asignaciones e indirección sin una necesidad
demostrada. La propuesta usa el puntero de función y el futuro boxed ya
existentes; su factibilidad de tipos y rendimiento todavía requieren evidencia.

Para aplicación se comparan los módulos privados actuales de World con una
frontera de crate: reward_commit todavía implementa WorldSession y el runtime
de trainer se adapta sobre WorldSession, aunque sus planes y puertos ya están
separados. Mantenerlos privados allí permite cortes físicos, pero no permite
ejercitar la operación completa sin importar el shell. El crate propuesto se
gana por esos coordinadores completos, sus consumidores World y los contextos
prestados/tests independientes; no por un número de líneas. No crea crates
por helper ni absorbe el driver, SQL adapters o almacenamiento canónico.

| Responsabilidad | Contrato propuesto y consumidores |
| --- | --- |
| Registro bajo, `wow-handler` | `PacketHandlerFn<S, C>` y `PacketHandlerEntry<S, C>` genéricos en receiver y catálogos; conservan opcode, status, processing, handler_name y una función registrada. El crate puede depender de `wow-constants`/`wow-packet`, sin World, Core ni dominios. |
| Handlers de dominio | Cada crate aporta `register<S, C>` con metadata y thunk en una sola declaración. Define sus contextos concretos por familia junto al handler; no recibe ni conserva WorldSession. |
| Construcción de contextos | Un contrato de host por dominio ofrece solo constructores de sus contextos ganados, parametrizado por los catálogos prestados. World implementa esos constructores mediante sus campos disjuntos; no son callbacks de gameplay ni un trait por helper. Ningún constructor devuelve el estado entero de la sesión, un mapa mutable o un guard síncrono que sobreviva a await. |
| Operaciones multidominio | Crear `wow-world-application` como frontera real de aplicación sobre Core y los dominios, con módulos privados por operación: login, loot, quest reward y trainer como primeras familias. Sus coordinadores y contextos no dependen de World. No contiene un estado universal ni otro servicio que replique los nueve estados. |
| Shell World | Driver, admisión, conexión y composición de préstamos. Mantiene los campos y su orden de Drop; los cambios de tipo/proveedor no adelantan destrucción ni mueven clocks. Sus adaptadores finales construyen contextos e invocan; no alojan el cuerpo de gameplay que supuestamente se retiró. |
| Composición de producción | world-server invoca los registrars y publica una tabla inmutable en SessionResources para las sesiones. world-modules conserva el compositor generado y llama a la misma composición mediante run_with_modules; no se edita su salida a mano. |

La firma baja propuesta conserva el lifetime compartido actual:

```rust
pub type PacketHandlerFn<S, C> = for<'a> fn(
    &'a mut S,
    &'a C,
    WorldPacket,
) -> HandlerFuture<'a, ()>;
```

Los thunks se monomorfizan para WorldSession y sus catálogos en la composición,
sin nombrarlos desde el dominio. El futuro sigue siendo Send y queda limitado
al préstamo de sesión/catálogos; no se hace static ni se crea otra tarea.
El dispatcher copia la entrada o sus valores antes de tomar el préstamo mutable
del receiver y espera su única función. No añade un clone de Arc por paquete.
Si la entrada implementa Copy/Clone, sus impls no exigen que S/C sean Copy/Clone.
La tabla se comparte al construir sesiones; no almacena S, C ni el agregado de
catálogos en el dominio. El builder no añade un segundo listado de opcode/call.
Los duplicados y las ausencias se rechazan en la composición y sus tests; el
conjunto válido mantiene exactamente el contrato anterior.

La composición sustituye también `construction.rs:114`, donde hoy cada sesión
vuelve a construir una tabla de referencias estáticas. Deben migrar juntos
`build_dispatch_table`, `get_handler`, `contains_handler`, la residencia del
driver, el dispatcher, SessionResources/session_factory y todos los tests que
leen `inventory::iter`. No queda un fallback de producción a un registro viejo.
Durante cortes internos, cada opcode tiene una única declaración y ruta activa;
la mezcla transitoria se retira antes del cierre. PacketHandlerEntry sigue
siendo la única fuente de metadata, admisión y llamada.
world-server añade dependencias normales directas de los crates cuyos registrars
invoca; no obtiene su metadata mediante un segundo agregador World.
Los dominios añaden la arista normal a wow-handler cuando contribuyen handlers;
application añade solo las aristas de sus operaciones reales, sin precargar
todos los dominios. World depende de application y construye sus contextos,
nunca al revés. Los gates de test-fixtures siguen separados de producción.
El handle de tabla compartida evita añadir un lifetime a WorldSession; se
obtiene una vez al construir la sesión, sin clone adicional por invocación.
Los tests de probe crean su tabla específica con el builder, sin mutar el
registro de producción compartido. El test de duplicados, el golden de metadata
y el snapshot de contratos se alimentan del mismo conjunto compuesto; no se
regenera su expectativa para esconder cambios. Los contextos tienen campos
privados y constructores ganados para el builder; no hacen públicos los states.

Primeras unidades completas y límites comprobados por fuente:

| Familia | Contexto y owner | Contrato que conserva el traslado |
| --- | --- | --- |
| Equipment-set assign/delete | InventoryState y acceso síncrono al Player canónico, con fallback fixture bajo su gate actual. No requiere el agregado de catálogos del driver. Los constructores deben estrechar el Hub actual a esa operación; save/use se analizan aparte porque consumen otros catálogos y publican. | Assign: LoggedIn/Inplace; delete: LoggedIn/ThreadUnsafe. Decode y errores actuales, sin respuesta inmediata ni await interno; dirty/tombstone se guarda después. Se preservan los seis escenarios de item_1 y se migran sus consumers; si siguen usando WorldSession permanecen como integración World. |
| Trainer buy | Decode/admisión NPC y planificación Spell; el coordinador de aplicación reúne Interaction, Spell, Lifecycle, dinero y publicación. Reutiliza el planificador wow-spell-acquisition y el contrato TrainerAcquisitionRuntimeLikeCpp existente; no crea un trait por fase. | LoggedIn/Inplace. Mantiene retiro de feign death, validación de procedencia, flush pendiente, exclusión de dinero y revalidación tras await. COMMIT/reconciliación antes de instalar el snapshot y publicar dinero; fence instancia→realm, visuales, fence realm→instancia, skills/acciones. La completion retiene la exclusión hasta tratar el resultado. |
| Quest choose reward | Un coordinador de aplicación conserva la operación normal completa, el plan durable, los estados participantes y el puerto PlayerQuestRewardPersistencePortLikeCpp. La proyección de quest pasa con sus lectores/escritores al módulo de quests de aplicación, sin duplicar el Player canónico. | Mantiene validación de choice/giver/inventario, mutaciones previas al COMMIT que ya existen, batch ordenado y aplicación/publicación posterior. Conserva cuarentena tras rollback/indeterminado y el money fence/cancellation fence; no impone commit-before-mutation a todo el inventario durante un refactor. |

Anclas de estas unidades en `a5f8da2eb`: delete está registrado en
`Server/Protocol/Opcodes.cpp:416`, llega a `CharacterHandler.cpp:1948` y
`Player.cpp:26524`. **Assign está STATUS_UNHANDLED/Handle_NULL en
Opcodes.cpp:170**, sin un método activo Player::AssignEquipmentSetToSpec en
ese checkout: se corrige la atribución Rust, se conserva su comportamiento
existente durante F5 y se registra la diferencia para el contrato/evidencia F6.
No se inventa paridad del opcode ni se declara una divergencia intencional aprobada.
Trainer está en `NPCHandler.cpp:132–202` y `Trainer.cpp:79–145`; quest reward
en `QuestHandler.cpp:396–403`, `Player::RewardQuest:14625` y su SaveToDB:14867.
El money fence Rust y el batch asíncrono no se prueban por la llamada síncrona C++.
La rama de quest reward sin dinero usa el testigo de quest para unknown-COMMIT;
la cancelación del futuro MariaDB en esa rama sigue sin evidencia específica.
Se preserva el código al moverlo y se retiene esa incertidumbre para F6/aceptación.

Secuencia dentro del mismo macro, sin nuevas micro-issues/PRs: fijar el registro
bajo y los consumers de composición; completar equipment-set como primera
familia de contexto/registro; trasladar trainer y quest reward con sus adapters
y pruebas; continuar las restantes familias mediante mapas de operación completos;
retirar los builders/thunks/bridges de origen y reconciliar los cortes diferidos.
Cada traslado consume el owner real y evita aristas dominio→aplicación/World.
Si una familia necesita varios dominios, se coordina arriba en aplicación;
no se añaden dependencias recíprocas para que compile. Los módulos nuevos
nombran responsabilidades; producción, fixtures y tests cumplen los presupuestos
físicos y cualquier excepción concreta conserva su salida acotada.

Aceptación de F5 dentro de la campaña de entrega de §10: conjunto exacto de
opcodes/metadata/handler_names/conexiones y llamadas antes/después, residencia
y fases, composición world-server y world-modules sin fixtures, módulos/hooks,
unión de identidades de pruebas sin pérdidas y configuraciones de cada crate.
Casos nuevos del registro prueban hosts no Copy, préstamos/cancelación, ausencia
y duplicidad, sin replicar gameplay. Se conservan los escenarios de trainer de
orden/writer detenido/fallo y los de recompensa de commit/rollback/indeterminado.
La evidencia exigida de persistencia, capturas y runtime sigue siendo necesaria;
las fixtures no demuestran el adaptador MariaDB ni durabilidad real. R1 conserva
5% + 300 y su estado rojo histórico; el objetivo R5 sigue siendo 20k–40k
producción, sin ocultar código en otro lenguaje, cfg o fixture. No se ejecutan
checks por estos cortes internos: se completan implementación/tests/consumers
y se conserva la campaña final y su presupuesto íntegro de 600 s.

**Aprobación de diseño — 2026-10-03, 10:25 UTC:** el usuario responde
«adelanteentonces» tras la explicación del registro genérico, contextos acotados
y nuevo crate de aplicación. Esa aprobación satisface la revisión explícita F5
conservada en §6; se continúa su implementación dentro de #1263 sin pedirla de
nuevo para cortes internos. Se mantienen la campaña diferida de aceptación,
R1/publicación y las autoridades separadas de runtime/Git. F4b sigue escrito
pero no aceptado; el diseño aprobado tampoco acredita implementación ni
aceptación de F5.

**Inicio de implementación F5 — 2026-10-03:** base local `fe059c1b0`,
sin ejecución de validación. La primera unidad define el registro genérico
en wow-handler y sus pruebas fuera de src. Para integrar las declaraciones
todavía residentes en World se usa una envoltura local temporal que contiene
únicamente una referencia a la entrada genérica estática, sin repetir metadata.
Es necesaria mientras inventory requiere un tipo colectable local a World;
el tipo genérico inferior no se convierte en un tipo local por instanciarlo
con WorldSession. Los macros temporales mantienen una única entrada y thunk
por declaración. La tabla compuesta consume esas referencias y los registrars
de dominio, con rechazo de duplicidad; la envoltura y su colección se retiran
cuando hayan migrado todos los opcodes. No son un segundo registro de fallback.
Los lectores de inventory, la prueba probe y los snapshots deben migrar a la
misma tabla efectiva, conservando sus assertions y gates. El constructor de
producción recibe la tabla de composición: no reconstruye una tabla parcial
por sesión. Este contrato de transición no acredita implementación aceptada.

**Cortes concretos en curso — 2026-10-03, 10:39 UTC (NO VALIDADO):**
equipment-set usa una capacidad prestada de Core con campos privados que
delegará la resolución generation-checked ya existente; no expone SessionCore,
Player ni el guard de mapa al handler. Su contexto de Inventory conserva el
fallback fixture únicamente cuando falta el handle, nunca ante un handle stale,
manager ausente, lock fallido o un resultado canónico `Some(false)`.
Las firmas antiguas Hub quedan como consumidores transitorios de ese contexto,
sin duplicar las reglas assign/delete. Save/use conservan su análisis separado.

La clausura de trainer requiere los módulos puros de preparación/traducción/
validación y el runtime genérico de spell acquisition, hoy residentes en World,
además de sus planes de profesión. El primer corte de aplicación traslada los
modelos y algoritmos puros de profesión con once pruebas; World conserva el
adaptador de snapshot y su escenario de sesión. Este primer consumidor normal
no es el coordinador de trainer completo: siguen pendientes su traslado, sus
contextos concretos, exclusión, fences y las tres pruebas de orden/cancelación.
El planificador inferior wow-spell-acquisition conserva sus modelos/planner.
La nueva arista de aplicación no autoriza dependencias de vuelta a World.

**Composición y primeros registros escritos — 2026-10-03, 10:55 UTC
(NO VALIDADO):** wow-handler aporta el registro genérico inmutable y cinco
pruebas, incluida la cancelación de un futuro pendiente que conserva solo su
primera mutación. La comparación de las 38 rutas World conserva las 397
expresiones originales al normalizar únicamente el macro de integración y el
lector inline Battle.net; después assign/delete trasladan dos declaraciones
a Inventory. El censo de fuente queda en 395 expresiones World, seis plantillas
y 87 invocaciones, más dos entradas de dominio: 478 esperadas, todavía sin
prueba del conjunto compilado. Su contexto consume una capacidad síncrona de
Core y sus métodos World quedan como adaptadores para los seis escenarios
existentes. Save/use permanecen pendientes de su corte de contexto.

world-server compone una tabla mediante el registrar de Inventory y los
registros World restantes, rechaza duplicidad y comparte el Arc por
SessionResources/constructor requerido. El campo conserva su posición; el
dispatcher copia la entrada antes del préstamo mutable y la fase mantiene su
FIFO. Los builders/getters World quedan bajo test/test-fixtures. La prueba
`production_linked_world_handler_registry_matches_snapshot` conserva su única
identidad y expectativa TSV al pasar de wow-world/tests a world-server/tests,
donde consume el compositor normal. En la campaña final se debe ejecutar
`cargo test -p world-server --test production_handler_registry_contract`
con PROTOC/jobs/target y presupuesto de §10; no se ha ejecutado ahora.
El traslado no sustituye la prueba aislada de features normales ni los demás
extras de persistencia, captura, runtime y arquitectura. Los controles de
registro requieren adaptar sus owners y gramática a estas declaraciones;
no se regeneran snapshots ni se omiten sus casos negativos para aceptarlas.

**Coordinación de trainer trasladada — 2026-10-03, 11:12 UTC
(NO VALIDADO):** aplicación contiene ahora el plan preparado, traducción,
validadores, runtime genérico y coordinador de compra completos, incluido el
DTO de oferta. Se retiraron sus proveedores anteriores de World; quedan
fachadas y los adaptadores concretos de sesión. La preparación del snapshot,
petición durable y puerto de persistencia siguen en World hasta cerrar sus
participantes. La comparación de fuente conserva los cuerpos del ejecutor,
preparación pura, cuatro módulos auxiliares y tres escenarios de trainer;
las once pruebas puras de profesión también conservan sus cuerpos. Esto no
demuestra compilación, paridad, cancelación ni durabilidad: quedan pendientes
los contextos concretos, exclusión monetaria, ambos fences y publicación.
Inventory conserva el argumento de estado del constructor para los fixtures;
el campo de respaldo solo existe con test/test-fixtures.

**Plan durable de quest reward trasladado — 2026-10-03, 11:16 UTC
(NO VALIDADO):** el módulo privado quests de aplicación contiene
`QuestRewardDurablePlanLikeCpp`; World conserva una fachada y los consumidores
reales en rewards, rewards/items y reward_commit. Sus siete campos privados y
nueve métodos mantienen su implementación al comparar fuente sin comentarios
ni promoción de visibilidad. El coordinador, commit y proyección de quests
siguen pendientes. La referencia C++ no significa una única transacción para
toda la operación: el correo opcional puede confirmar antes de su
`SaveToDB(false)` final; el comentario del proveedor conserva ese límite.

**Instalación canónica de spell acquisition — 2026-10-03, 11:22 UTC
(NO VALIDADO):** `OwnedSpellAcquisitionAccessLikeCpp` mantiene privado su
préstamo mutable a Core. Recibe filas canónicas de spells; su conversor sigue
en Spell para evitar Core → Spell. Conserva validación del snapshot temporal,
conversión de skills, invalidación y aplicación generation-checked sin await.
World conserva la rama fixture exacta y sincroniza el directorio después de
éxito; ese paso sigue pendiente de sus participantes Loot/Social. Las fuentes
`Player.cpp::AddSpell:2741–3139` y `SetSkill:5635–5853` delimitan el
comportamiento base, sin demostrar equivalencia de esta preparación async.
Los escenarios existentes de skill owner, spell state y effect learning y
los casos de entidad permanecen escritos; no se han ejecutado en este corte.

**Preparación y clasificación durable trasladadas — 2026-10-03, 11:35 UTC
(NO VALIDADO):** aplicación contiene las cuatro funciones restantes de
spell acquisition: detección de save pendiente, autoridad estable privada,
petición durable y ejecución/reconciliación a través del puerto. World retira
el proveedor prepare anterior y el enum de resultado, conservando fachadas.
La comparación de fuente conserva los cuatro cuerpos, las cuatro salidas y
la reconciliación únicamente tras COMMIT desconocido. Se trasladan, con sus
identidades y cuerpos, los dos casos independientes de autoridad estable y
resultado del puerto, junto con su mock y petición fixture; World conserva
los demás escenarios de sesión. No cambia SQL, la exclusión, el cancellation
fence ni la cuarentena; esos participantes concretos siguen pendientes de F5.

**SaveEquipmentSet trasladado — 2026-10-03, 11:44 UTC (NO VALIDADO):**
Inventory contiene el contexto privado completo, decodificación, validación,
normalización y publicación de EquipmentSetId tras la mutación. World conserva
la fachada con generador y retira su declaración; el registrar Inventory
aporta ahora Save/Assign/Delete con sus metadata existentes. Core ofrece
lectura de conjuntos, snapshot de inventario y colecciones, y publicación por
el canal existente. Los constructores solo reúnen referencias, preservando
los clones y lecturas por slot/apariencia; la reconstrucción fixture de cada
participante conserva un único proveedor. Las fallas con handle presente no
habilitan fallback y `Some(false)` sigue siendo un resultado canónico.
Use, sus validaciones pendientes de F6 y sus consumidores continúan en World.
El conjunto compilado, bytes, estado dirty y save durable siguen sin aceptar.

La capacidad de spell acquisition pasa a préstamo compartido de Core al
confirmar que su invalidación solo muta Player mediante el guard canónico;
cambian las firmas, no los cuerpos, llamadas, locks ni fallas. Esto permite
componer lectores, pero la cuarentena aún escribe el estado de sesión y
necesita un límite concreto antes de cerrar el contexto completo de trainer.

**Mutación de inventario sin Hub — 2026-10-03, 11:54 UTC (NO VALIDADO):**
El provider existente delega en un acceso prestado a la autoridad de inventario.
Conserva la invalidación por GUID antes de resolver la mutación estricta por
handle generacional, el callback síncrono, las ramas handle-less de fixtures y
su reflejo posterior. Las dos reconstrucciones iguales de fixture comparten
un helper privado; no se añaden copias, locks ni una autoridad paralela. Los
consumidores mantienen su fachada actual mientras se cierra Use y modificadores.
No se ejecutó compilación ni aceptación.

**Persistencia monetaria con acceso acotado — 2026-10-03, 11:58 UTC
(NO VALIDADO):** Lifecycle recibe solo identidad y cuarentena de Core para
la tarifa de trainer sin adquisición durable y su clasificador de resultado.
Las fachadas Hub delegan en los mismos cuerpos; no cambian overrides de
fixtures, igualdad de saldo, request, exclusión ni cancelación. Ambos caminos
indeterminados de trainer desconectan dentro del commit: el de adquisición
permanece en World; el monetario conserva mark-indeterminate, disarm, kick,
warn y retorno en ese orden. No se pospone la cuarentena al resultado del
executor. El contexto completo de trainer y su aceptación siguen pendientes.

**Providers de modificadores sin Hub — 2026-10-03, 12:05 UTC
(NO VALIDADO):** Inventory delega ocho providers en una capacidad tipada de
Core: snapshot, añadir/quitar item y bonus de set, retirar set vacío, aplicar
acción y reset. Se mantienen el handle generacional estricto, las ramas de
fixtures sin handle y los retornos anidados del remove; el reset limpia su
registro fixture antes de mutar. No añade invalidación, clones ni otro
acumulador. El productor conserva sus lecturas de catálogos y orden de acciones;
su contexto completo y Use siguen pendientes. Referencia de la operación:
`Player.cpp:7654::_ApplyItemMods`, `:7688::_ApplyItemBonuses` y
`:7975::_ApplyWeaponDamage` en `a5f8da2eb`; estas capacidades no acreditan paridad.

**Consultas de adquisición sin Hub — 2026-10-03, 12:19 UTC
(NO VALIDADO):** la capacidad de spell acquisition añade presencia canónica,
concesión de dual wield y snapshot de tombstones. Las dos primeras conservan
el fallback por GUID; tombstones conserva el handle estricto y su único
fallback fixture cuando falta el handle. Spell y Hub delegan sin cambiar sus
firmas ni la instalación completa. La comparación de fuente no sustituye
pruebas ejecutadas. `SpellEffects.cpp:2176::EffectDualWield` en `a5f8da2eb`
tiene el gate HIT_TARGET y `SetCanDualWield(true)`; no demuestra la admisión
ni la durabilidad del executor de trainer.

**Commit combinado de trainer en aplicación — 2026-10-03, 12:30 UTC
(NO VALIDADO):** el contexto privado reúne Lifecycle y el acceso monetario
acotado de Core; World conserva su fachada. El cuerpo trasladado mantiene
override de test, clon del puerto, GUID, fence, token, request y await, y los
cuatro resultados. Indeterminate conserva mark-indeterminate, disarm,
quarantine, warn y retorno dentro del commit. El override no existe en la
firma normal; bajo test-fixtures World pasa None salvo su cfg(test) original.
Application añade únicamente Core/Lifecycle y rand/tracing usados por esta
operación, sin dependencia hacia World ni una arista inversa de Lifecycle.
La metadata incluye constants/entities y el gate de fixtures del corte Quest
en curso; ese corte no queda cerrado por guardar sus dependencias. Persisten
los escenarios World y los tres tests del executor, todavía sin ejecutarlos.
El contexto completo de trainer y la evidencia DB/cancelación siguen pendientes.

**Publicación completa de skills sin Hub — 2026-10-03, 12:45 UTC
(NO VALIDADO):** Core contiene el contexto privado de publicación con los tres
stores y cuatro referencias fixture seleccionados. El publisher conserva
GUID y stores, snapshot, sort, límite de 256, máscaras completas, lecturas de
raza/clase/nivel por fila y map ID al enviar. Hub mantiene su firma y delega.
Los lectores compartidos de identidad y registros conservan un único provider:
identidad usa el fallback fixture incluso con handle stale; registros solo lo
usan sin resultado canónico y sin handle. El fallback normal de nivel sigue
disponible bajo Core test-fixtures para consumidores normales de Inventory.
`Player.cpp:5635::SetSkill` y `:25723::_LoadSkills` en `a5f8da2eb` son las anclas
revisadas; no prueban bytes nuevos ni la ordenación representada del bridge.
Los callers y escenarios World se conservan sin ejecución nueva.
La instalación exacta de skills también tiene un único cuerpo en Core, con
las dos referencias fixture seleccionadas. Su módulo, método y fachada Hub
siguen bajo el gate original test/test-fixtures: no se añade una ruta normal.
Se conservan la conversión previa de filas, el clon de tombstones dentro de
la mutación canónica y el mirror sin handle, incluido loaded && complete.
Esta continuación revisada en fuente a las 13:20 UTC permanece NO VALIDADA;
no aporta evidencia nueva de compilación, pruebas ni paridad.

**Sincronización y publicación sin Hub — 2026-10-03, 13:22 UTC
(NO VALIDADO):** Core contiene la operación completa de publicación de posición
en el registro, con seis referencias fixture prestadas y lectores compartidos
de posición, vitales, nivel y transporte. Se conserva el orden de GUID,
posición, registro, mapa, vitales, instancia y construcción del update; las
fachadas Hub delegan y mantienen sus fallbacks originales. La capacidad de
publicación contiene los dos fanouts completos, con las mismas selecciones
de destinatarios, clones, conexiones y comandos; conserva los fences existentes
y el resultado del envío de valores, leyendo el mapa en el punto de envío.
Anclas contrastadas en a5f8da2eb: Player.cpp:6122::UpdatePosition,
Unit.cpp:12257::UpdatePosition y :11566::SendPlaySpellVisualKit,
Object.cpp:1746/1752::SendMessageToSet y Player.cpp:6141/6173.
Estas anclas no prueban la equivalencia del directorio Rust, sus fallbacks
legacy ni los fences asíncronos. La hidratación de fixtures, loot y party del
sync completo y el contexto de trainer siguen en curso; no se ejecutaron
compilación, formato, pruebas, capturas ni QA runtime por este traslado.

La continuación revisada en fuente a las 13:54 UTC cierra el sync completo
en `application/registry_sync.rs`: conserva la prueba exterior de GUID/registro,
la publicación de posición con su prueba propia, la hidratación World cfg(test),
el snapshot de identidades de loot sin ordenar y la publicación de party con
una prueba nueva. Core guarda el binding del control channel en una capacidad
privada, sin devolver el registro; Social delega al mismo publisher de party.
La hidratación conserva orden, duplicados y puntos de clon de spells, quests,
vehículo y pet, incluyendo la mutación por GUID. Sus tres puntos de llamada
World permanecen; el lector normal de quest conserva el camino estricto y la
fixture solo se selecciona sin handle bajo el gate World original.
Los escenarios de registro, party, quest y loot permanecen integrados en World.
La unidad queda escrita y congelada, **NO VALIDADA**; no acredita ejecución de
esas pruebas, paridad del directorio ni terminación del contexto de trainer.

**Commit de recompensa y barrera monetaria en aplicación — 2026-10-03,
13:54 UTC (NO VALIDADO):** `application/quest/reward_commit.rs` contiene el
cierre completo del plan durable con Inventory, Lifecycle, Quest y acceso
privado al Player. La moneda usa un único lector/escritor canónico y el plan
por CurrencyTypesStore conserva el orden de iteración y cambios de flags sobre
la copia. Las fachadas World no anticipan queries ni clonan Player o catálogos.
El port y el override no-I/O se resuelven en el punto anterior; ese override
conserva el gate World cfg(test). La rama monetaria reutiliza el clasificador
Lifecycle y su testigo de dinero; la rama sin dinero conserva el testigo de
quest, sus resultados y cuarentena. La aplicación de memoria sigue al COMMIT
conocido y precede al drop de la exclusión.
`application/quest/money_persistence.rs` comparte begin/reconcile con todos
sus callers World. Conserva los dos clones/esperas del tracker, cierre de
admisión, compare_exchange antes de leer dinero, límite, MoneyChanged solo
cuando cambia el saldo, cuarentena y adquisición del mutex al final. No drena
criterios ni añade un evento al setter de dinero. Se conservan los escenarios
World de transacción única, rollback, COMMIT perdido y quest repetible en
`unit_tests/handlers/quest_tests/reward_transaction.rs`, sin ejecutarlos.
Anclas de cierre: RewardQuest llama a SaveToDB en Player.cpp:14867 y el save
llama a _SaveCurrency en :19654, en a5f8da2eb; no prueban los contratos
asíncronos. El cuerpo de _SaveCurrency está en :6800–6845 y SaveToDB en
:19312/19323; se distinguen de esos sitios de llamada. La incertidumbre de cancelación
sin dinero y el resto de la operación normal choose-reward permanecen abiertos.
No se ejecutaron compilación, formato, suites, scanners, capturas ni QA live.

**Contexto del trainer admitido y runtime compartido — 2026-10-03,
14:15 UTC (NO VALIDADO):** el ejecutor normal ya usa `AppTrainerCx`,
con campos privados y préstamos disjuntos de Core, Inventory, Lifecycle,
Spell, Quest y Loot, cinco stores seleccionados y las referencias fixture
existentes. Sus constructores no resuelven Player ni ejecutan queries.
`session/trainer_acquisition.rs` construye el contexto; el handler lo usa en
un bloque y después trata la completion conservando la exclusión monetaria.
Stage, publicación de dinero, fences de los dos writers, visuales y skills
usan los providers únicos revisados. El commit sin adquisición conserva el
gate test-fixtures del Lifecycle anterior; el commit combinado conserva el
override World cfg(test). No se modifica el archivo Lifecycle en este corte.

La instalación de adquisición y la publicación de sus acciones tienen un
proveedor compartido por trainer y EffectLearning. Las fachadas World reciben
los mismos iteradores y delegan sin materializarlos antes, conservando los
rechazos tempranos. La rama fixture valida todas las filas antes de invalidar,
consulta primero el runtime canónico estricto y después usa el fallback bajo
el gate World original; conserva las conversiones, fallback rows y evidencia
de trait-config. El snapshot representado clona sus campos dentro del mismo
préstamo canónico, mediante una proyección síncrona tipada, sin clonar Player
ni exponerlo. Tras instalación correcta se ejecuta el sync completo del
registro antes de las acciones. El fallback de tombstones conserva por
separado su gate Core test-fixtures, sin añadirle el gate World de instalación.
Los escenarios existentes del ejecutor App y de trainer/acquisition World
permanecen montados; no se ejecutaron pruebas ni se atribuye evidencia nueva.
Anclas: Trainer.cpp:79–145, NPCHandler.cpp:132–202 y
Unit.cpp:11566::SendPlaySpellVisualKit en a5f8da2eb. No prueban los fences ni
durabilidad asíncronos. Admisión NPC, planificación, revalidación y saga de
battle pets continúan en World; este contexto no termina la familia buy ni F5.
Compilación, formato, ownership, composición, capturas y aceptación siguen pendientes.

**Dinero y banco con acceso canónico acotado — 2026-10-03, 13:07 UTC
(NO VALIDADO):** OwnedInventoryAccess concentra las siete lecturas/mutaciones
de dinero, cantidad y flags de bolsas bancarias y cantidad de slots de
inventario. Inventory conserva una implementación por provider, con fachadas
Hub y consumidores nuevos con capacidad. Getters mantienen el fallback solo
tras None y sin handle; setters conservan el mirror fixture y sus resultados,
incluido el índice inválido de flags. Las ramas cfg no llaman a operaciones
de fixture desde la compilación normal. Anclas: Player.cpp:23376::SetMoney,
:9424::SetInventorySlotCount y Player.h:1332–1335 en a5f8da2eb. No se cambiaron
publishers, ejecutaron pruebas ni demostraron nuevas garantías de durabilidad.

**Proyección y publicación de inventario completas — 2026-10-03, 13:39 UTC
(NO VALIDADO):** Core envuelve el clon existente de Player en una proyección
opaca de planificación, sin exponer Player, Deref ni autoridad canónica nueva.
Inventory contiene el contexto privado y el cuerpo completo del snapshot y
su publicación; World construye capacidades y presta los dos stores de objetos.
Se mantienen el snapshot completo incluso para coinage, la reconstrucción de
fixtures solo sin handle, las lecturas y clones de runtime, la mutación canónica
única de visible items antes del snapshot y el resultado real del envío.
La conversión de ItemStorageTemplate tiene un solo provider con stores elegidos.
Banco, almacenamiento, equipo, preflight, carga/persistencia y void storage
consumen la misma proyección; los closures sobre Player canónico conservan sus
métodos originales. Las fachadas y los escenarios existentes permanecen;
no se movieron ni retiraron pruebas en este lote. Anclas contrastadas en
a5f8da2eb: Object.cpp:190::BuildValuesUpdateBlockForPlayer,
Player.cpp:3634::BuildValuesUpdate, :11502::SetVisibleItemSlot y los métodos
CanStoreItem/CanBankItem/CanUseItem/CanEquipItem/CanUnequipItem/SwapItem.
El traslado delega a las operaciones Rust existentes: no prueba su paridad
ni repara las diferencias retenidas para F6. Compilación, formato, suites,
capturas y aceptación siguen pendientes junto con el resto de F5.

**Planificación y aplicación de modificadores de objetos — 2026-10-03,
12:59 UTC (NO VALIDADO):** Inventory contiene la operación completa con un
contexto privado, dos capacidades canónicas y seis referencias de catálogo.
World conserva sus fachadas; los providers compartidos de scaling, tipo de
inventario, resistencia, shield block y límites de daño tienen una sola
implementación. Se mantienen el clon del runtime/item, el único clon del Arc
de stats, las lecturas repetidas en sus puntos originales, la salida temprana
sin runtime y el orden feral/ataque/forma antes de aplicar las acciones.
Los fixtures de nivel/forma se prestan; evidencia solo se registra cuando el
consumidor World está bajo cfg(test). Los ocho providers canónicos existentes
conservan sus cuerpos. Player.cpp:7654::_ApplyItemMods,
:7688::_ApplyItemBonuses y :7975::_ApplyWeaponDamage en a5f8da2eb son las
anclas contrastadas, no prueba de paridad de toda esa cadena. El bridge Rust
mantiene sus lecturas de bounds antes de feral/desarme; C++ aplica primero
la compuerta feral/apply/desarme. No se repara esa diferencia en F5 ni se
atribuyen a C++ los fallbacks Rust de propietario ausente. Los consumidores
y escenarios existentes se conservan, sin ejecutar compilación, formato ni
pruebas.

**Proyección de misiones en aplicación — 2026-10-03, 12:55 UTC
(NO VALIDADO):** SessionQuestState contiene los siete campos normales privados
y sus fixtures en módulos privados de estado, rewards y sharing. Los lectores,
escritores, inicializadores y consumidores World pasan por sus operaciones;
World conserva las lecturas canónicas y los gates cfg(test) originales. Los
cuatro registros de evidencia reciben el gate del consumidor explícitamente,
sin confundir test-fixtures del crate destino con tests del origen. El drenado
mantiene su latch, el orden pop/await y el estado tras cancelación; el dinero
solo encola un cambio tras una mutación aceptada. La hidratación de completed
bits itera por préstamo, sin un clon nuevo del conjunto.
Se migraron los consumidores de los 43 archivos de pruebas World afectados.
La comparación textual de declaraciones de funciones de esos archivos no
encontró cambios de identidad; no fue un listado compilado ni una ejecución.
Se conservan los anclajes de QuestHandler.cpp:396–403 y
Player.cpp:14625::RewardQuest en a5f8da2eb y el límite XP de F6. El coordinador
completo de recompensa, sus participantes concretos y su aceptación siguen
pendientes. No se ejecutaron Cargo, suites, formato ni QA live.

**QA del registro genérico — 2026-10-03, 11:51 UTC (NO VALIDADO):**
La política de handlers usa schema 2: varias rutas explícitas de registro y un
único dispatcher; solo se añade Inventory como owner ya implementado. El
scanner reconoce el puente legacy completo de World (wrapper local, collector,
macro y reexport exactos), conserva el rechazo de aliases, cfg, collectors
ajenos y montajes ambiguos, y distingue los seis templates existentes de las
invocaciones directas cualificadas. El positivo de `register_move` conserva el
cuerpo real con `$opcode`; las repeticiones y entradas reenviadas se rechazan.
No se ejecutó el checker ni se cambió el snapshot. Este checkpoint no
acredita el conjunto compilado ni aceptación.

**Cierre de fuente del builder y composers — 2026-10-03, 12:53 UTC
(NO VALIDADO):** el checker analiza el registrar real de Inventory, sus
entradas completas y el receiver del builder. La comprobación de repositorio
consume también los dos composers reales y las fachadas públicas exactas:
cuatro exports en la raíz y tres desde equipment_sets. Rechaza registros
condicionales, duplicados, alias, forwarding y miembros extra en esas fachadas;
los aliases de tipos que solo describen metadata no se cuentan como registros.
Las pruebas privadas escritas cubren las fuentes reales y mutantes positivos y
negativos; conservan los seis templates y el snapshot de 478 filas. No se
ejecutaron esas pruebas, el checker, Cargo ni formato.

**Familia Instances y registro compartido — 2026-10-03 (NO VALIDADO):**
base local `7e734878c`. La operación completa de raid-info, reset, respuesta al
pending-bind, extensión y selección de dificultad reside en módulos privados de
Application. Tres contextos prestados reúnen solamente sus participantes; World
construye esos préstamos e invoca la operación. El consumidor del comando de grupo
también usa el mismo cuerpo de aplicación. Los siete registros de raid-info,
reset, respuesta y dificultad se declaran en `instances/registration.rs`; los
dos composers llaman a ese registrar después de Inventory y antes del puente
legacy. La extensión conserva su entrada/alias en Loot, sin reparar protocolo.

Core conserva las capacidades privadas del Player, grupo y manager de locks,
sin devolver manager, Arc, guard ni Player. El reset y raid-info usan el schedule
default anterior; bind y extensión leen el schedule configurado en sus puntos
originales. El guard síncrono termina antes de persistir o enviar paquetes.
Se conservan la mutación previa al commit, los clones anteriores, el orden de
miembros y el primer comando de persistencia de dificultad. El contador de
rechazo sigue en la fixture Combat existente; su incremento y el registro de
confirmación conservan el gate World cfg(test). Los fallbacks de preferencias
y GM mantienen el gate test/test-fixtures de sus providers originales; el de
group-guid mantiene el gate del consumidor World. No se añade otro estado.

Los consumers y escenarios World existentes se conservan. Se escribió una
prueba independiente del registrar con un host no Copy y constructores que
no deben invocarse durante el registro: primera composición y duplicidad.
La QA de source usa contratos finitos de Inventory e Instances, sus fachadas
exactas y ambos composers; conserva los casos sintéticos independientes y los
rechazos de aliases, ausencias, duplicados, type-arguments y montajes cfg.
No se modificaron las expectativas del snapshot de opcodes ni el baseline.

Anclas C++ en `a5f8da2eb`: `GroupHandler.cpp:594::HandleRequestRaidInfoOpcode`,
`MiscHandler.cpp:890/910/968/1061`, `CalendarHandler.cpp:532`,
`Player.cpp:19006::ConfirmPendingBind` y `:20667::ResetInstances`.
Las diferencias de pending-bind, completed-mask, opcodes y extensión se retienen
en F6 abajo; esta extracción no las acredita como paridad. No se ejecutaron
Cargo, formato, suites, checker, capturas ni QA live. La evidencia aceptada
anterior no se atribuye a estos cambios y R1 conserva su estado rojo histórico.
Trainer buy todavía depende del traslado del guardado completo previo a la
compra; quest reward depende también de los providers completos de inventario
y XP/estadísticas. Las demás familias y la aceptación de F5/F6 siguen abiertas.

**Estadísticas y dependencias de guardado/XP — 2026-10-03, 18:03 UTC
(implementación y revisión de fuente, NO VALIDADO):** base local `fcaa65781`,
con cambios todavía en el árbol de trabajo. Application incorpora el módulo
privado `stats` y `CharacterStatsApplicationCxLikeCpp`: proyección, actualización
normal, conservación de porcentaje de salud, refill de nivel y snapshot de login.
World conserva adaptadores y los puntos de llamada de Create/Login/XP/equipo/auras.
Core aporta `PlayerStatsAccessLikeCpp`, con identidad y once referencias concretas
de Combat/Aura para fixtures; construirlas no captura el Player ni sus auras.
Los fallbacks conservan sus gates y la ausencia del handle; las mutaciones y la
publicación siguen separadas por la liberación del guard canónico.

La revisión encontró y corrigió un método de Inventory fuera de su bloque `impl`.
También corrigió consultas por ranura/GUID que copiaban mapas completos al
adaptar `equipment_slots`: ahora usan consultas tipadas de una entrada y los
mismos mapas fixture bajo ausencia del handle. La fachada de deltas de nivel
pasa por el contexto de Application, evitando un método privado de Core y un
préstamo de un Hub temporal. El mapeo puro de tipo de poder por clase pasa a
definición/exportación normales, conservando su cálculo y los gates de otros
símbolos. El bloque tiene cierre de revisión de fuente y se conserva en un
checkpoint local **NO VALIDADO**; no hay candidato aceptado. Las fachadas de
multiplicadores que consume `scaling.rs` permanecen explícitas. No se modificó
ningún archivo de pruebas: el consumidor de deltas en
`wow-world/unit_tests/handlers/character_tests/skill.rs` ya usa sesión mutable.
Los consumidores se inspeccionan por fuente; no se ejecutaron Cargo, formato,
suites, scanners, captura ni QA live.

XP completo y RewardQuest siguen en implementación. La transición de nivel y
talentos requiere un acceso con referencias obligatorias y evaluación de puntos
de quest en su fase original; la revisión rechaza callbacks a World y campos
opcionales que omitan efectos. `wow-script` pasa a dependencia normal de
Application para la llamada existente de GiveXP, con lock/política ajustados;
la nueva dependencia no demuestra que la ruta completa esté trasladada.
SaveToDB también sigue parcial: captura/recibo están en Core y la persistencia
en Application, pero quedan el coordinador entero, las completions de loot y el
drenaje de objetivos. Sus operaciones completas tienen un único implementador;
el preflush de Trainer no se sustituye por un callback de World.

Revisión de continuación, 2026-10-03 18:27 UTC, sobre el árbol posterior a
`bf0b48038` (sin commit ni aceptación de estos cambios): los inputs del save
son ahora referencias prestadas e inertes. Core materializa tutoriales después
de void storage y antes de instance lock times, y calcula el tiempo transcurrido
después de esos locks y antes de reputaciones. El recibo conserva la copia de
tutoriales del request; la rama fixture pasa `&expected` al cálculo de grupos.
Estas correcciones se inspeccionaron con `sed`/`rg`; no se ejecutaron checks.
El coordinador entero y las dos operaciones pendientes siguen sin acreditarse.
La fase de estadísticas XP exige las referencias seleccionadas y presta el
owner, pero el bucle `give_xp_runtime_like_cpp` y su wrapper asíncrono permanecen
en World. Trainer List necesita todavía proveedores reales para admisión NPC
y retirada/publicación de feign death; publicar la lista construida no cierra
esa operación. Estos límites no reducen la entrega F5/F6 ni sus gates.

Revisión de la dependencia aura de Trainer, 18:30 UTC: el consumidor
`session/movement/state.rs::remove_represented_feign_death_if_needed_like_cpp`
selecciona slots, invoca `remove_aura` y después limpia DIED. No basta con
eliminar el slot y publicar AuraUpdate. El cuerpo existente
`session/spell_state/aura_application.rs::remove_aura` también retira transform,
sincroniza threat y, después del paquete, evalúa por spell/catálogo escalado de
item level, efectos porcentuales de estadísticas, velocidad de ataque,
shapeshift y display power. La selección FeignDeath no prueba que esas fases
sean omitibles. El proveedor nuevo debe conservar esas decisiones y su orden;
la inspección de código no acredita todavía su implementación o aceptación.
Decisión parental tras el mapa de fases (2026-10-03, NO VALIDADO): conservar una
única operación App de retirada completa, reutilizable desde Trainer y World,
con Spell/Inventory y capacidades Core seleccionadas prestadas. Las fases de
mount/control, speed/presentation y shapeshift/item-stats se organizan en módulos
privados; Stats se toma prestado temporalmente en su fase. Una especialización
erase-FeignDeath omite
efectos por spell y conservar callbacks World impide retirar esa responsabilidad,
por lo que ninguna satisface F5. La retirada y aplicación recursivas reutilizan
la misma autoridad, sin nuevo lock, mirror o contexto universal. Primero se
traslada la selección/removal/rollback y transform; la ruta de producción no
delega al nuevo cuerpo hasta cerrar todos sus efectos. C++ SpellAuraEffects.cpp:
2189–2260::HandleFeignDeath, SHA a5f8da2ebf5424bf0450ca4e08843ecbf72577bd, limpia
flags y DIED en su rama remove; ese anchor no demuestra que los demás efectos
del coordinador Rust sean omitibles ni acredita sus gaps de paridad.
Primera fase escrita y revisada por fuente (2026-10-03, NO VALIDADO): App/
aura_removal/initial conserva consulta mounted, presentación actual, retirada,
reinserción ante fallo de presentación y transform posterior. Core usa referencias
fixture seleccionadas y consulta canónica antes del fallback NoHandle; Spell y
las fachadas existentes delegan esos providers. El cuerpo App sigue privado y
no sustituye la operación World completa: el cierre de todas las fases continúa
pendiente. No se ejecutaron checks.
La fase threat también está escrita: conversión de spell a u32 antes de mutar,
retirada canónica única y fallback NoHandle limitado al consumer World cfg(test),
con recorrido de los 32 bits del effect-mask. World/aggro delega solo su rama
apply=false al mismo provider; el cuerpo Aura completo continúa pendiente.
Mount/control está escrito y su provider seleccionado ya existe como archivo.
La ausencia de mod/reexports de aura_removal detectada en la revisión de
integración se corrigió en Core/canonical_access y Core/session: ambos roles
se exportan normalmente y los tres tipos fixture bajo cfg(any(test, feature
= "test-fixtures")). Se verificó por fuente, sin compilación. Las fachadas
no acreditan el cierre del remover completo. La comparación
por fuente con HEAD de World/player_presentation, Core/movement/movement_publication,
Core/movement/state y Core/spell_state/cast conserva presentación antes del cálculo,
mutación canónica de altura, mirror limitado al consumidor World cfg(test), consumo
del contador antes del gate scale-duration y publicación posterior del estado de
movimiento con radio/realm-connection originales. El provider de mascota conserva
lectura de GUID, modo y lifecycle, mutación antes de PetMode y limpieza final de
temporary-react incluso sin mascota. Esto no cierra la recursión del remover entero:
velocidades, publicación de aura, stats, shapeshift y display siguen pendientes de
integración completa y aceptación. Las pruebas negativas solicitadas no se han ejecutado.
El montaje Core y los dos casos escritos en World/scenarios_pets_1 se revisaron:
ausencia de mascota conserva lifecycle salvo temporary-react y no publica PetMode;
handle obsoleto rechaza presentación sin consumir el contador fixture. Falta el
caso de consumo del contador antes de scale-duration ausente, según el implementador.
Contrato de velocidad contrastado con C++ SpellAuraEffects.cpp:3159–3257, SHA
a5f8da2ebf5424bf0450ca4e08843ecbf72577bd: DecreaseSpeed actualiza run, swim,
flight y las tres velocidades backward en ese orden. IncreaseFlightSpeed calcula
la velocidad antes de la rama de flags; el cuerpo Rust World actual hace flags
antes del recálculo. El gate de gravedad no es una carencia: al seguir el caller
hasta Core/movement/fall.rs::move_represented_player_fall_like_cpp se confirma
el rechazo DISABLE_GRAVITY antes de FALLING, también conservado en el provider
nuevo. La ausencia de un gate duplicado en el caller no acredita un defecto.
La fase App/movement_speeds y el provider Core/mount_control/speed ya están
escritos. La revisión preserva conjuntos de efectos y ramas independientes,
dos consultas de auras para Fly y MountedFlightSpeed, y el orden current-rate,
mutación, propagación a pet, GUID/opcodes, forced-counter, lectura actual de
velocidad, movement-counter, paquete propio, MovementInfo actual y broadcast.
Se corrigió por fuente el import ObjectGuid del provider. Estos cuerpos no
acreditan el remover entero: siguen pendientes publicación, stats, shapeshift,
display y recursión integrada, además de la aceptación no ejecutada.
La fase privada publication ya contiene AuraUpdate de retirada, gate LoggedIn
antes del predicado de total-stat, preservación de health y ataque. La revisión
de los providers conserva slot/aura_data=None/update_all=false, máscara de efecto,
predicado de habilidad/stamina y consultas actuales de attack-speed/autoattack
antes de una mutación canónica. Integrar Stats requiere reborrow temporal de las
referencias de aura; dos roles simultáneos que muten/lean esos mismos fixtures
no se consideran una solución. El coordinador entero sigue sin cerrar.
Contrato de shapeshift revisado por fuente: World/spell_state/aura::sync_form_ownership
clona primero SpellStore, consulta el efecto, resuelve mutated-form y vuelve a
consultar presencia del spell. Si queda otra aura del mismo spell devuelve Applied,
incluso durante RemoveAura; no es válido implementar solo Removed. La rama restante
lee otro snapshot y elige la primera forma en iteración nativa antes de set-form.
World/character/stats::sync_form exige base-attack-time antes de boosts y retorna
si falla; después ejecuta display-power, refresh de item-effects y Stats. Retirar
boosts exige el remover entero por spell/slot, conservando los snapshots y conteos.
C++ SpellAuraEffects.cpp:1838–1866, SHA a5f8da2ebf5424bf0450ca4e08843ecbf72577bd,
comprueba prev-form antes de aplicar boosts y solo limpia la forma si no queda
MOD_SHAPESHIFT; Player.cpp:22090 conserva display antes de equip-spells y damage.
Estos anchors no convierten la proyección Rust en paridad demostrada. La extracción
debe preservar la operación representada completa y separar su revisión F6.
Las ramas privadas Applied/Removed de boosts y display-power ya están escritas
y revisadas por fuente. Applied conserva known-spells actuales, clonación del
vector nativo y atributos por difficulty; Removed conserva slots y recursión.
Display conserva default de clase, consulta actual de efectos, cálculo canónico
y publicación solo tras cambio. C++ Unit.cpp:5550–5603 incluye ramas vehicle/pet
y UpdateDisplayPower llama SetPowerType; su ancla no demuestra paridad completa
del cálculo Rust representado. FullApply/FullRemove y refresh de item-effects
siguen pendientes de integración, sin checks ni pruebas ejecutados.
El contrato de refresh completo se contrastó con World/player_items/equipment:
snapshot íntegro de items, slots de auras incompatibles ordenados, retirada de
todas esas auras, items ordenados por slot/GUID, aplicaciones completas y por
último item-set refresh. C++ Player.cpp:8158–8245 retira/aplica por cada item
admitido y comprueba auras activas por cast-item durante formChange. La secuencia
Rust de retirar todas antes de reaplicar todas se conserva en F5 y requiere
contraste F6; no se presenta ese traslado como paridad nueva.
El módulo privado item-effects ya contiene refresh, aplicación inicial y replay
de item-set completos. La revisión conserva snapshots íntegros, ordenaciones,
clones y gates tardíos, planificación nativa y conteo de eventos separado del
éxito de Apply/Remove. Sigue pendiente conectarlo al coordinador íntegro de
auras; esta revisión por fuente no ejecutó pruebas ni acredita esa integración.
El sync-form privado ya enlaza base attack-time, ambas ramas de boosts,
display-power, refresh íntegro y reborrow temporal de Stats, con retorno temprano
original si falla base attack-time. FullApply/FullRemove siguen pendientes.
Los cuerpos FullApply/FullRemove ya están escritos y revisados contra World/
aura_application:146–303 y :500–706, con sus fases y publicación originales.
También está escrito el enlace al escalado completo y el constructor inerte
de roles, catálogos seleccionados y fixtures disjuntos. Stats se reborrowa
temporalmente y Registry usa las referencias actuales, sin copiar autoridad.
Falta conectar constructores World, consumidores públicos y casos completos;
los cuerpos nuevos siguen NO VALIDADOS y no cierran aún Aura/FeignDeath/Trainer.
La revisión posterior del cuerpo original Inventory/storage:616–626 detectó
dos lecturas reducidas en la extracción: initial-item equip y su gate de loaded
mods usaban el lector directo de Item, mientras el original proyecta runtime
completo antes de clonar Item. Se asignó conservar ambas lecturas independientes
mediante el proveedor full-runtime existente; no reutilizar un item precalculado.
ItemTextQuery ya usa ese proveedor completo y su wire se contrastó con C++
QueryPackets.cpp:495–519: valid-bit, longitud/texto de 13 bits y GUID, incluido
texto vacío de respuesta inválida. Ninguno de estos contrastes ejecutó pruebas.
Ambas lecturas de equipo ya se corrigieron por fuente al proveedor full-runtime.
El constructor World y las fachadas completas de Apply/Remove ya están escritos,
con export normal App; aún faltan casos completos, FeignDeath y consumidores Trainer.
FeignDeath ya está escrito y World delega: consulta DIED por la misma mutación
canónica, snapshot actual y slots nativos, retirada completa por slot con
resultado ignorado y limpieza DIED al final. Sigue sin prueba ejecutada ni
aceptación de todos los consumidores Trainer.
El modo de manejo y la inmunidad polymorph/Dragonmaw de la rama mounted-flight
requieren contraste completo en F6; la extracción F5 conserva el cuerpo Rust,
sin introducir esas reparaciones dentro del traslado.

ItemSet, continuación 18:41 UTC (NO VALIDADO): están escritos el acceso Core
`canonical_access/item_sets.rs` y el proveedor Inventory `item_sets.rs`, con
montajes privados y constante única exportada por Inventory. El factory Core
acepta referencias de catálogos y tres inputs fixture seleccionados, sin
resolver Player en el constructor. La revisión corrigió los cinco helpers
privados usados desde un módulo hermano, el nombre duplicado del método de
retirada y dos préstamos de `hub.shared()` temporales. La consulta de límite
heirloom quedó unificada en el proveedor tipado, con fachada Inventory y sin
la fachada World sin consumidores. La constante conserva una fachada privada
para el test `scenarios_player_items_7`, que la consume mediante `super::*`.
Los cuerpos Add/Remove se contrastaron por inspección; no se ejecutaron
compilación, formato, pruebas ni aceptación. UseEquipmentSet entero permanece
pendiente. El stub NPC que retiene HubRef no está aprobado para integración:
debe sustituirse por referencias seleccionadas y una única implementación de
las decisiones de admisión, sin duplicar ni reconstruir el Hub.

Contraste acotado con Trinity `a5f8da2eb`: `Entities/Player/Player.cpp:2247`
(`GiveLevel`), `Entities/Unit/StatSystem.cpp` (`UpdateStats`/`UpdateAllStats`) y
`Spells/Auras/SpellAuraEffects.cpp:3656` (`HandleModTotalPercentStat`). El delta
de nivel Rust compara dos filas de stats y base mana; C++ compara los valores
nuevos con `GetCreateStat`/`GetCreateMana` y calcula nuevos talentos, además de
otras fases de GiveLevel. El traslado conserva la implementación Rust y no
demuestra esas fases ni repara las diferencias: quedan en el contrato de F6.
Todo F5/F6 y su aceptación siguen abiertos.

**Acceso NPC seleccionado — 2026-10-03, 18:56 UTC (NO VALIDADO):**
`wow-world-core/session/canonical_access/trainer_npc.rs` ofrece
`NpcInteractionAccessLikeCpp`, con factory público, tres referencias de catálogo
y doce inputs fixture obligatorios mediante constructor opaco. No retiene ni
reconstruye HubRef/SessionFixtures. Los cuerpos únicos de NPC y reacción están
en `npc_interaction.rs` y `faction_reactions.rs`; sus fachadas Hub delegan sin
duplicar las decisiones. La revisión corrigió los campos privados inaccesibles
desde esos módulos hermanos. La consulta de reputación sigue tras liberar el
guard canónico; el branch legado conserva su orden existente.
Se contrastaron `Player.cpp:1929–1980::GetNPCIfCanInteractWith` y
`Entities/Object/Object.cpp:2709–2857::GetReactionTo/GetFactionReactionTo`, SHA
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`. C++ comprueba reacción antes de
distancia; Rust conserva el cálculo de distancia dentro del lookup y la reacción
tras soltar ese guard. El traslado no corrige ese orden ni acredita paridad de
los inputs representados, owners, flags o fallback legado. Compilación, pruebas,
capturas y aceptación no se ejecutaron. Trainer List/Buy completos siguen
pendientes de aura, controller y preflush.

**Integración en curso — 2026-10-03, 19:19 UTC (NO VALIDADO):**
La revisión de fuentes detectó y confirmó corregidos el movimiento de una
referencia mutable antes de registrar la finalización de una misión
(`quest/completion.rs`), la importación de fixtures XP sin su cfg, y las
referencias duplicadas de nivel/vitales en el contexto XP. La finalización
conserva `Option<Option<u8>>`: la existencia del propietario y el resultado
de la transición siguen siendo hechos distintos; la compatibilidad de tests
se actualiza antes de aplanar el resultado. Esto todavía no demuestra el
flujo completo de completion, visibilidad, recompensa automática y drain.

El nuevo `equipment_set_use.rs` contiene decode, bucle de selección, movimientos,
sincronización Registry, actualización Stats/fallback y respuesta. La fachada
World ya delega el handler; el constructor de combate usa directamente Core
para no devolver un préstamo de un propietario local. Los préstamos Stats
se construyen desde campos disjuntos con vidas explícitas. Faltan la revisión
completa de consumidores, registros y pruebas de esta unidad; no se declara
aceptada por la presencia del cuerpo o de la fachada.

Se contrastó `xp_gain.rs` con el código anterior de `progression_adapters.rs`
y `Player.cpp:2176–2246` (`SetXP`/`GiveXP`) del SHA C++ ya registrado. Se
preservan clamping, dirty/scaling y el fallback de tabla existente; el nuevo
acceso no demuestra por sí solo paridad del GiveLevel completo. El proveedor
de condiciones todavía necesita terminar su proyección y consumidores y
retirar los cuerpos duplicados de género/objetos de condición. Ninguna de
estas revisiones ejecutó Cargo, tests, validadores ni QA runtime; F5/F6 y
la aceptación del macro siguen pendientes.

La inspección posterior de visibilidad encontró un paso que no puede sustituirse
por la conversión directa de bytes: `session/publication/operations.rs` filtra
los flags NPC mediante `represented_viewer_dependent_creature_npc_flags_like_cpp`,
que llama a `represented_can_see_spell_click_on_creature_like_cpp`. Aunque el
primer gate de refresh solo comprueba la existencia de condiciones, este paso
posterior construye objetos y snapshots, aplica reglas de usuario/reacción y
evalúa condiciones con contexto vivo. `Hidden` retira SPELLCLICK; `Visible` y
`ExactContextUnrepresented` conservan el flag. F5 debe mantener ese contrato
completo, los clones y el orden; no sustituirlo por un filtro fail-closed nuevo.
El borrador de envío directo fue retirado y el traslado completo sigue pendiente.
La dependencia normal/fixtures App → `wow-world-entities` queda declarada en
manifest, lock y policy por los imports reales del nuevo módulo de visibilidad.

Checkpoint UseEquipmentSet (2026-10-03, implementación NO VALIDADA): App posee
decode, selección por slot, movimiento completo, Registry, Stats/fallback y
respuesta; Inventory posee los dos movimientos y reutiliza búsqueda por GUID,
snapshot de bonuses y búsqueda de backpack. Las fachadas World y la composición
de `world-server` usan el registrar App con LoggedIn/Inplace y el mismo nombre
de handler. El checker tiene un descriptor finito propio y casos escritos para
ausencia, aliases y procedencia/composición incorrectas; no se ejecutaron.
Los cinco escenarios World existentes de uso de conjuntos conservan identidad.
Las raíces y archivos compartidos se guardan selectivamente, sin incluir los
cambios concurrentes de recompensas, condiciones, guardado o XP.
Se leyó `CharacterHandler.cpp:1953–2011::HandleUseEquipmentSet` y
`Opcodes.cpp:1005`, SHA `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`.
F5 conserva el algoritmo Rust representado; el flujo completo C++ de
CanStoreItem/CanUnequipItem/CanEquipItem, nested inventory y fanout no queda
acreditado por ese traslado. Compilación, contratos ejecutados, capturas y
aceptación F5/F6 siguen pendientes.

Revisión de integración XP/condiciones (2026-10-03, NO VALIDADO): el borrador
RAF de `quest/reward/xp_grants.rs` conserva la secuencia Rust de
`session/social/contacts.rs::gets_recruit_a_friend_bonus_like_cpp(true)`:
admisión, nivel, pertenencia al grupo, posición/mapa/instancia y recorrido con
relectura de límites y relación de reclutador. El acceso Core verifica ambos
registros antes de consultar el grupo y libera su guard antes del recorrido.
Queda pendiente sustituir el cuerpo World compartido por un único proveedor
para XP y los consumidores con `for_xp = false`, sin duplicar su política.
El rechazo Rust de miembros muertos no acredita el cálculo C++ desde el cadáver.
El filtro spell-click escrito aún debe materializar la proyección de condiciones
después de clonar PlayerConditionStore y antes de clonar AreaTableStore; recibir
un holder precomputado no conserva ese orden. La valoración total/equipada ya
tiene ownership exclusivo asignado para cerrar esa dependencia. Estas revisiones
de fuente no acreditan compilación, ejecución ni la entrega completa de F5/F6.

La proyección de condiciones ya tiene un constructor inerte y un cuerpo App
para las lecturas anteriores a los niveles de equipo. La revisión detectó un
lifetime omitido del store de áreas en el constructor. El contraste completo de
known-spells aclara el camino World cfg(test)/sin handle: su snapshot consulta
Core antes del fallback fixture y reconstruye el runtime mediante
`wow-world-spell/records.rs:112::canonical_player_spell_runtime_like_cpp`.
`wow-entities/player/spell_runtime.rs:400::install_acquisition_snapshot_like_cpp`
asigna la lista conocida sin filtros ni reordenación. Por tanto no se requiere
inventar un filtro para la colección fixture ni omitir la consulta Core anterior;
el proveedor debe conservar esa secuencia y su gate de consumidor World cfg(test).
Las dos lecturas tardías de valoración y sus consumidores siguen pendientes.
El adaptador RAF de `World/session/social/contacts.rs` ya delega en el cuerpo
App compartido con `for_xp`; los filtros condicionales de XP y los consumidores
sin ese filtro conservan una única implementación en el borrador. El flujo
principal de `World/session/xp_grants.rs` y el helper de request de
`World/session/quest/persistence.rs` todavía requieren sustitución completa.
El acceso Core de valoración escrito conserva solo una referencia privada a
SessionCore y devuelve lecturas acotadas sin retener guards. Sus lecturas de
PvP y capacidades de equipamiento aún deben convertirse en la implementación
única reutilizada por las fachadas Inventory existentes, incluidos los callers
offhand y el fallback sin handle de capacidades `(false, false)`. Este borrador
no demuestra los promedios completos ni sustituye CanUse/Unique/CanEquip.
El request App escrito conserva rest-state, flags, bonus, nivel y XP en ese orden;
queda pendiente comprobar sus llamadas reales y retirar el cuerpo duplicado.

Revisión de guardado completo (2026-10-03, NO VALIDADO):
`session/lifecycle/persistence.rs::save_current_player_to_db_with_generator_like_cpp`
todavía conserva el coordinador World; `Application/player_save.rs` solo posee
captura/persistencia. La revisión de `Core/player_save_owner.rs:279–323` confirma
que el recibo se consume una vez, contrasta handle y GUID, accede a la misma
incarnación y aplica la intersección de grupos esperados/confirmados antes del ACK.
`World/.../persistence/prepared.rs:112–140` publica tutoriales únicamente tras
ese ACK y conserva el dirty cuando sus valores cambiaron; después sincroniza
Registry para spells/skills. La extracción completa debe mantener esa publicación
antes de liberar el bloqueo de dinero, liberar luego la admisión y finalmente
drenar objetivos; las ramas de cuarentena y snapshot no disponible también
deben conservar su drain posterior a la liberación. Falta cerrar el coordinador
y ejecutar evidencia de cancelación, COMMIT desconocido e incarnación obsoleta.

Revisión de cuerpos de valoración/XP (2026-10-03, NO VALIDADO): Inventory
`valuation/item_level.rs` contiene los borradores completos de ItemLevel y
AvgEquipped. El contraste mantiene el orden PvP/caps/templates/override/curva,
bonuses/clamp y la relectura por item del promedio equipado. Se detectó un import
de SessionCatalogs desde el crate equivocado y se asignó su corrección. AvgTotal
y la cadena de equipabilidad siguen pendientes. World `session/xp_grants.rs`
ya delega las dos entradas XP al contexto App mediante referencias fixture
seleccionadas; aún faltan el cierre del helper de persistencia y la revisión
de consumidores y módulos. El lifetime del store de áreas de la proyección
de condiciones está corregido. No se ejecutaron compilación ni aceptación.

Revisión de consumidores XP (2026-10-03, NO VALIDADO): el adaptador del request
de persistencia había cambiado su receptor World de `&self` a `&mut self` para
construir QuestRewardCx. La corrección escrita conserva ya `&self` y delega en
un proveedor App de lectura seleccionado, sin prestar participantes mutables
innecesarios. La operación XP se está separando en hojas privadas de RAF, reposo
y persistencia; los helpers llamados desde el padre necesitan visibilidad
`pub(super)`, cuyo cierre se ha asignado sin ampliar campos públicos.
La evidencia existente a ejecutar incluye los escenarios de script/zero/max-level,
reposo, RAF, campos canónicos y conexión Realm de `scenarios_misc_8.rs`, y
`lifecycle_persistence/player_persistence.rs::represented_xp_reaches_the_port_for_every_classified_outcome_like_cpp`
para Applied/Failed/Unknown. Su existencia no constituye resultado ejecutado.

Revisión de cierre de dependencias (2026-10-03, NO VALIDADO): el import de
SessionCatalogs en Inventory/item_level está corregido al owner Core. La
proyección App ya puede consumir el snapshot tipado existente
`Inventory/storage.rs::resolved_inventory_items_with_access_like_cpp`, que
conserva su único kernel y la copia del inventario, ahora público para el
consumidor concreto, sin exponer mapas mutables ni crear otro estado. El campo
`consumer_test` ya tiene el cfg correcto. Se detectó un cfg accidental en el
campo normal `area_table_store` y se asignó retirarlo, pues su construcción y
uso son de producción. El filtro App ya llama `project_like_cpp` entre las
copias de PlayerConditionStore y AreaTableStore; el método completo todavía
depende de la valoración pendiente. No se ha iniciado aceptación.

Dependencia completa de AvgTotal/CanEquip (2026-10-03, diseño de cierre F5):
`World/player_items/equipment.rs::can_equip_inventory_item_like_cpp` consulta
`modifiers.rs:21–50::item_limit_category_template_like_cpp`, que materializa
la proyección completa de condiciones cuando existe el store de condiciones,
incluso sin filas aplicables. Esa proyección consulta AvgTotal al final. Inventory
no puede llamar Application para resolverlo sin invertir la dependencia.
El coordinador completo AvgTotal/CanEquip se asigna por ello a una hoja privada
Application de valoración. Ya está escrito el catálogo prestado seleccionado
`Core/catalogs/inventory_valuation.rs::InventoryValuationCatalogViewLikeCpp`,
con campos privados y sin clonar Arcs ni tomar snapshots; ItemLevel/AvgEquipped
lo consumen en lugar del catálogo completo. Inventory conserva esos cuerpos y kernels
de almacenamiento/equipabilidad. Application materializa las condiciones en la
fase original y pasa el contexto al kernel de cantidad de límite. No se admite
callback World, una dependencia Inventory → Application ni omitir/cachear el
promedio para ocultar la reentrada. La posible recursión del código previo se
conserva como límite de comportamiento pendiente de contraste F6. La proyección
de condiciones llamará AvgTotal App y después AvgEquipped Inventory por separado.

Revisión de reposo XP (2026-10-03, NO VALIDADO): la hoja privada App conserva
el consumo incondicional mediante SetRestBonus incluso con premio entero cero,
el orden bonus/modificador y la máscara `0x07` cuando cambian threshold o estado.
El contraste con `World/test_support/operations.rs:237–293` detectó que el
borrador agrupaba las dos relecturas finales en una tupla: debe retornar si
threshold no está disponible antes de leer estado, como el cuerpo original.
La corrección está asignada; el snapshot inicial de la envoltura async sí tiene
evaluación de tupla en el original y debe conservar ambas lecturas. La existencia
de estas diferencias de fase se comprueba por fuente, sin ejecución de tests.

Integración del filtro spell-click (2026-10-03, NO VALIDADO): la fachada World
ya delega el filtro completo a App y construye capacidades seleccionadas inertes;
el snapshot Domain se llama directamente sin split Hub. La revisión detectó dos
tipos fixture referidos desde el namespace World sin reexport; se asignaron
imports gated directos de Core en el archivo consumidor. La proyección completa
de condiciones y la publicación de visibilidad aún requieren cierre. En reposo
XP ya se corrigieron las dos guardas secuenciales finales y el let-else inválido.
Estos resultados son inspección de fuente, sin compilación ni captura.

Revisión del owner de guardado (2026-10-03, NO VALIDADO): el borrador añadió
cuarentena al acceso de captura que conserva `&SessionCore`, aunque `Core::kick`
requiere `&mut self` para pasar a Disconnecting. La corrección asignada mantiene
la API de captura de lectura. El owner opaco mutable ya está escrito para el
coordinador completo, con reborrow de captura en su fase tardía y cuarentena
mediante receptor mutable. La factory ya liga explícitamente los lifetimes de
Core y sus dos stores. No debe debilitarse kick,
exponerse Core a App ni cambiarse la firma de los consumidores de captura.
La proyección de condiciones ya llama AvgTotal App antes de AvgEquipped
Inventory, con guardas separadas y sin adelantar snapshots; la hoja AvgTotal y
sus constantes todavía no están escritas, por lo que esa llamada sigue siendo
una dependencia de fuente incompleta. El helper World de flags de reposo ya
delega al proveedor App de lectura compartido, preservando su receptor `&self`.

Revisión de continuidad (2026-10-03, NO VALIDADO): se verificaron los cuatro
handles de implementación. Trainer estaba idle tras una revisión de firma que
no necesitó cambios; se reanudó su entrega de List/Buy completos. Guardado ya
delega la persistencia y cuarentena al acceso mutable App, pero la coordinación
de admisión, loot, reconciliación, bloqueo, captura y drenaje permanece World:
esa delegación no demuestra traslado completo de SaveToDB. En XP, el helper RAF
privado tiene ahora visibilidad de padre correcta; los dos cuerpos fixture de
SetRestBonus/consumo aún duplican las transiciones App. Se asignaron solamente
esas dos fachadas World al dueño de XP para conservar un único proveedor y el
orden de consultas. No se ejecutaron compilación, tests ni aceptación.

La revisión siguiente del catálogo de valoración confirma por fuente que map,
faction y friendship seleccionan los mismos stores que los getters existentes.
Los helpers Core de bonus de nivel, bonus PvP y efectos ya delegan al nuevo view
sin duplicar sus algoritmos. En Inventory todavía hay dos cuerpos del cálculo
de curva (valuation.rs y valuation/item_level.rs); se asignó su consolidación al
mismo dueño. La hoja App AvgTotal sigue ausente en esta observación: no hay cierre
de condiciones ni evidencia de compilación de ese consumidor.

Revisión de confirmación de guardado (2026-10-03, NO VALIDADO): Core intersecta
grupos esperados/confirmados y exige handle y GUID actuales antes de resolver el
Player por su handle en Map. La nueva capacidad mutable reusa esa confirmación.
El consumidor World solo marca tutorials cargados tras insert confirmado y solo
limpia dirty si los valores actuales coinciden con el receipt; Registry se publica
después. Los tests existentes de Entities cubren filas modificadas durante el
commit, pero no prueban el enlace Core/Session de handle obsoleto ni tutorials
posteriores. Esas regresiones de integración se asignaron al dueño de Save,
manteniendo pendiente el traslado del coordinador completo y su aceptación.

El caso existente `scenarios_persistence_3.rs:283`,
`player_save_plan_marks_dirty_state_only_after_commit_like_cpp`, comprueba dirty
antes/después de la confirmación mediante el shim fixture y sin modificar
tutorials entre captura y confirmación. Se conserva; no sustituye las dos
regresiones del receipt canónico identificadas arriba ni requiere duplicarse.

Primera implementación App AvgTotal (2026-10-03, NO VALIDADO): la hoja ya está
escrita. La revisión de su scan confirma dos pasadas, snapshot de objetos antes
del inventario directo, ItemLevel antes de elegibilidad, fast-path de slot
equipado y ajuste de arma a dos manos conservados frente al cuerpo World.
Faltan montar la hoja, enlazar su entrada con `project_like_cpp` y cerrar los
proveedores de elegibilidad; el cuerpo World original aún no está retirado.
Esta observación sustituye la ausencia registrada arriba, sin demostrar cierre
del traslado ni compilación. Se notificaron firma/mount a sus dueños disjuntos.

La revisión de CanUse contra `World/player_items/persistence_load.rs:9–154`
detectó dos diferencias de ese borrador: primary specialization debe salir del
mismo Player snapshot inicial, no de una segunda consulta de la capacidad;
el fallback de spells de un consumidor World cfg(test) debe propagarse con
feature test-fixtures y su bool consumidor, no con cfg(test) del crate App.
Ambas correcciones se asignaron antes de retirar el cuerpo original. Los
valores constantes de otros argumentos ya estaban presentes en ese original;
conservarlos en F5 no establece su paridad C++.

Dependencia de almacenamiento de Reward (2026-10-03, NO VALIDADO):
`player_items/persistence.rs:455–652` consulta limit-category después de construir
overlays, posiciones vacadas, templates y referencias de almacenamiento. Esa
consulta requiere el proveedor de condiciones App; la coordinación completa se
asigna a App y los kernels de almacenamiento a Inventory. Se preservarán las
fachadas y consumidores vendor/void/reagent del plan existente. Reward y CanEquip
deben compartir ese proveedor, sin copia parcial ni callback de vuelta a World.
La asignación cubre la operación y sus wrappers, no otras mutaciones del archivo.

Revisión de dependencias de valoración (2026-10-03, NO VALIDADO): el nuevo
CanUse ya toma primary specialization del snapshot original y el helper de
spells aplica el bool del consumidor dentro del gate test-fixtures. En el nuevo
provider de reputación, la conversión de standing a rank aún ocurre dentro del
guard del Player; el original Inventory/modifiers.rs:426–457 la hace después de
terminar la consulta de standing. Se asignó restaurar ese alcance y delegar el
helper antiguo al único proveedor. Su fallback any(test, test-fixtures) sin
handle coincide con Core/progression/reputation.rs:301–319 y debe conservarse.

Integración en curso (2026-10-03, NO VALIDADO): el módulo App de valoración ya
está montado y su entrada libre toma prestado el contexto de condiciones. Se
detectó un nombre de constructor incorrecto y se asignó corregirlo. La revisión
también detectó ediciones de valoración en archivos de condiciones cuyo dueño
era trainer: se detuvieron nuevas ediciones de esos archivos por valoración y
se devolvió su continuidad al dueño, preservando el trabajo ya escrito. El
helper canónico de Save tiene dos definiciones temporales durante su conexión;
se asignó consolidarlas y conservar el resultado de snapshot no disponible en
todos los consumidores. Ninguno de estos borradores demuestra compilación.

Primer coordinador App Reward (2026-10-03, NO VALIDADO): ya existe el cuerpo
ordenado de la operación en `quest/reward/coordinator.rs`, incluidas retirada,
grants, commit, settle, GameEvent, paquetes, XP y delay-teleport final. Conserva
el retorno de cuarentena sin reset de delay cuando settle falla después del
commit. Los proveedores auxiliares todavía requieren cierre e integración.
La revisión detectó que el nuevo helper de dinero no propaga el abort original
si falta el saldo; se asignó conservar esa guarda antes de continuar el plan.
La búsqueda del slot recompensado también debe enlazarse al proveedor original,
no suponerse un método del estado fixture. World aún conserva el coordinador;
no se reclama traslado completo ni evidencia de compilación o paridad.

Handoff posterior de Reward (2026-10-03, NO VALIDADO): el implementador entrega
los cuerpos completos de requisitos/turnins, grants fixed/chosen/package y
almacenamiento, reputación, QuestLog y coordinador con sus fachadas World.
La revisión conjunta detectó un bloqueo de integración de manifest:
reward/reputation.rs usa wow_progression::mgr::SetReputationOptionsLikeCpp,
Application/Cargo.toml no declaraba esa dependencia normal. El implementador
la añadió y se verificaron por fuente tanto la declaración normal como la
entrada wow-world-application del lockfile, sin ejecutar Cargo ni
comprobar todavía el grafo efectivo. Las completions anidadas de objetivos
siguen a cargo de su propietario; la entrega Reward no acredita esos consumidores.
El siguiente bloque Inventory asignado es la operación completa de area-scaling,
incluidos remove/apply mods, restauración de health y publicación condicional,
reutilizando Stats y los roles seleccionados. C++ Player.cpp:28715–28729,
SHA a5f8da2ebf5424bf0450ca4e08843ecbf72577bd, confirma remove, activate, apply
y restore; la fórmula entera/max(1) Rust se conserva en F5, sin afirmar equivalencia
de redondeo con el porcentaje float C++. No se ejecutaron pruebas ni metadata.
InventoryScalingApplicationCx y el constructor World ya están escritos. La revisión
de fuente conserva map/PvP antes de using-flag, vitals y targets actuales antes de
remove/set/apply, restauración seguida de Registry y publicación solo cuando publish
y targets no vacíos. Se corrigió el fallback que invocaba un método ausente tras
mover el contexto Stats: ahora usa el rol de publicación disjunto, con GUID, snapshot
completo de bonuses y map actuales después del fallo de la proyección Stats.
El montaje/reexport App ya está conectado. Ya se escribieron
en World/scenarios_player_items_2 dos casos reales de la operación: rechazo del
owner obsoleto con reemplazo de igual GUID sin mutación/paquetes, y cambio con
publish=false que exige remove/apply en orden, health restaurada y retry sin
nuevos eventos. El segundo caso se amplió con Registry real del fixture y un cambio
vehicle-kit que debe publicarse incluso con publish=false. También se escribió
fallback crudo condicionado por publish y targets no vacíos. El implementador
entrega la unidad conectada por fuente, NO VALIDADA; ninguna prueba fue ejecutada.
La próxima operación Inventory asignada es CancelTempEnchantment completa, incluida
la aplicación de plan con condiciones/socket-context y registro tipado LoggedIn/Inplace.
C++ ItemHandler.cpp:1100–1116, SHA a5f8da2ebf5424bf0450ca4e08843ecbf72577bd,
confirma equipped-slot, item/enchantment, ApplyEnchantment(remove) y ClearEnchantment.
La dependencia Rust Inventory/enchantment.rs:205–260 retira el objeto antes de leer
required-skill; su retorno si falta esa lectura debe conservarse en F5 y revisarse
como diferencia F6, sin ocultar una reparación dentro del traslado.
El owner Core de enchantment y el template compartido ya están escritos y
exportados. La revisión conserva skill-records completos y su proyección,
fallback NoHandle original y mutate_canonical_player, sin reemplazarlo por
una mutación de otra autoridad. Inventory/enchantment/operation contiene la
operación completa de condición/socket/plan: lecturas actuales, remove antes de
skill, mutación y reinserción. Los lectores seleccionados de item/slot conservan
la proyección completa de runtime que usaban las fachadas originales. El handler
CancelTempEnchantment y su registro ya están escritos y conectados: context
Inventory, host genérico, constructor World y fachada de compatibilidad. La
entrada conserva opcode, LoggedIn, Inplace, nombre y warning de lectura; se retiró
la entrada del collector World y la composición server usa el registro Inventory.
La revisión del cuerpo conserva conversión/slot, GetPos y runtime actuales,
enchantment no nulo y ambas mutaciones con sus resultados ignorados. Se preservan
los casos World existentes. El implementador entregó además tres casos nuevos:
registro Inventory exacto y entrada única en la composición, lectura inválida y
slots fuera de rango frente al slot válido, y owner obsoleto tras reemplazo de
igual GUID. Se revisaron por fuente; ninguno se ejecutó. La unidad queda cerrada
por implementación, NO VALIDADA, y conserva pendiente la aceptación completa.
La siguiente operación asignada es ChangeBankBagSlotFlag, con admisión bancaria
y publicación VALUES completas. El target C++ Opcode.cpp:289 registra ese opcode
STATUS_UNHANDLED/Handle_NULL; F5 conservará el comportamiento Rust y deja esa
diferencia para F6, sin atribuirle un handler C++ inexistente.
El cuerpo App/bank y su host/thunk tipado ya están escritos, con constructor
World de roles seleccionados. La revisión conserva source GUID, GUID actual,
admisión NPC BANKER y publicación del snapshot completo de Player VALUES.
Registro/composición únicos, consumidores y casos nuevos siguen en integración;
el cuerpo escrito no se cuenta como aceptación del handler completo.
Se escribieron tres escenarios de registro exacto, payload/límites y admisión
self-GUID con enable/disable, y rechazo de owner obsoleto tras reemplazo del mismo
GUID. No se ejecutaron. La revisión de consumidores requiere también compositor
normal world-server, compositor de fixtures y contrato finito del scanner para
el nuevo registrar Bank; añadir el cuerpo no sustituye esas conexiones.
Las conexiones ya están escritas: root App, compositor normal world-server,
compositor de fixtures y retirada de la entrada World anterior. El scanner
declara Bank como contrato finito de package/module/host/facade; sus fixtures
incluyen el nuevo módulo y negativos de owner/host/alias. La integración normal
añade una aserción de entrada única y metadata sin regenerar el TSV. Revisión
por fuente positiva; handoff y aceptación ejecutada siguen pendientes.
El implementador entregó ChangeBankBagSlotFlag completo y congelado por fuente,
sin dependencia ni consumidor pendiente identificado; corrigió también el
conteo previo del registro Inventory de tres a cuatro. Sigue NO VALIDADO.
La continuación Inventory asignada es ItemTextQuery completo: objeto íntegro
actual, respuesta válida con texto o inválida, envío y registro único tipado.
C++ QueryHandler.cpp:305–318 y Opcodes.cpp:550, SHA
a5f8da2ebf5424bf0450ca4e08843ecbf72577bd, fijan GetItemByGuid/Text y LoggedIn/Inplace;
faltan traslado, consumidores y evidencia de esa siguiente operación.
ItemTextQuery se entregó completo por fuente: context Inventory, World thin,
registro único de cinco entradas, consumidor normal y contrato/fixtures finitos.
Dos casos nuevos del thunk cubren parse-failure, bytes válidos/ inválidos y
owner reemplazado con igual GUID; están escritos, no ejecutados. No hay nuevas
dependencias ni mounts pendientes identificados. La siguiente responsabilidad
asignada es el mapa acotado de swap/equip completo y sus fences/await/publicación,
para definir su traslado sin callbacks World ni duplicar la operación existente
de UseEquipmentSet. Ningún cierre por fuente acredita la aceptación de #1263.
El contraste inicial ubica Player::SwapItem en Player.cpp:12271 y el handler
en ItemHandler.cpp:130–173 del SHA target. El handler C++ admite source/destination
bank por separado; Rust combina ambos con una sola consulta. Real-swap Rust
espera persistencia y trata Failed/Unknown juntos antes de efectos, publica
posiciones/items y luego consulta loot actual, libera loot, actualiza TitanGrip/
item-level/Registry y ejecuta child/offhand. El mapa debe conservar esas fases
y contrastar el caller entero antes de atribuir fences ausentes. Las diferencias
se separan de F5. UseEquipmentSet tiene un controlador síncrono distinto:
reutilizar roles/proveedores no autoriza sustituir esa operación por su handler.
El mapa acotado de validate-target y sus tres planes confirmó que todas las
consultas seleccionadas necesarias ya existen. Se escribió un único contexto
App de planificación de solo lectura sobre PlayerCondition, con Bank completo
en hoja privada y una definición única de target. World ya tiene las cuatro
fachadas thin. La revisión conserva rereads completos, count/destino exactos,
capabilities/in-combat actuales, cache y listas nativas y contains-items tardío.
Anchors de kernel: Player.cpp:9615/9882, :10584 y :10820. Root/export están
conectados y se revisaron cuatro casos escritos: planes positivos sin mutación,
split/exactitud, fuente ausente/NotEquippable y reemplazo de owner con igual GUID.
Este último demuestra primero Store/Bank válidos con fuente fixture sin handle,
antes de exigir que el handle retirado rechace las cuatro operaciones sin fallback
ni publicación. No se ejecutó validación. Sigue pendiente la ejecución completa
swap/equip; el siguiente mapa asignado cubre real-swap, sus callers y todos los
efectos anteriores y posteriores al await, no solo la planificación.

El siguiente corte de efectos ya está conectado por fuente: App privado
`inventory_swap/effects.rs`, contexto seleccionado con constructor inerte y
dos fachadas World en `session/player_items/storage.rs`. Remove mantiene
duration/tradeable antes del gate de equipo, ItemSet, consulta FULL runtime
de broken, mods, clear-equipped/enchantments y profesión-stat; Store mantiene
duration, gate, equipped, ItemSet y una nueva consulta FULL runtime de broken
antes de mods. `None` conserva decisiones distintas en remove/store. Los
proveedores de duración/tradeable usan operaciones Core finitas; ItemSet
conserva su lector directo y eventos actuales, sin añadir aplicación de auras.
Cuatro casos escritos en `scenarios_player_items_2.rs` cubren duración y paquetes,
modificadores/clear, ItemSet activo en broken y owner reemplazado con igual GUID.
Revisión por fuente favorable; no se ejecutaron. C++ Player.cpp:11553 y :7654
del SHA target muestran responsabilidades adicionales de auras/meta/enchantments
que no se reparan dentro de F5. Committed-swap, publicación, StorageMove,
child/offhand y release-all siguen pendientes de traslado completo.

CommittedSwap también está conectado por fuente: App
`inventory_swap/committed.rs`, export normal y fachada World de cuatro argumentos.
Conserva GetPos source/destination, FULL InventoryItems/ItemObjects, children
sin ordenar, writes top-level independientes, GUID fresco posterior, updates
source/destination/children y cierre Core finito de placement nativo. No agrupa
guards ni elimina escrituras del original. Cuatro casos escritos cubren swap
top-level, bolsas canónicas con children y bag_slot trasladados, destino ausente
y rechazo del owner reemplazado con igual GUID. El caso de destino ausente
retorna en GetPos; no demuestra el gate posterior de container. Revisión por
fuente favorable; sin ejecución ni paridad nueva. El siguiente corte asignado
es publicación completa de campos de item y slots de container, conservando
lectores FULL, masks y Map tardío; publicación de posiciones/stats sigue aparte.

Ese grupo de publicación ya está conectado por fuente en Inventory
`publication.rs`/`storage_bags.rs`, con lectores y canal Core seleccionados.
Las fachadas Hub comparten el mismo cuerpo y World relocation/dynamic y
real-swap usan los proveedores finitos; child → old-bag → new-bag conserva
su orden. Los publishers Core consultan MapID al final y usan `send_packet`
como el original. Tres casos escritos en `scenarios_player_items_2/item_publication.rs`
construyen masks/bytes esperados, cambian MapID entre envíos, verifican clear
del slot viejo y children frescos, y rechazan owner stale sin fallback ni
paquetes. Revisión por fuente favorable, sin ejecución. El siguiente corte
asignado es PositionPublication: sort/dedup → bolsas inmediatas → lecturas
top-level frescas → Player VALUES → Stats completo, con vista de bolsa canónica
que conserva su accessor actual (no strict-owned ni fallback nuevo).

PositionPublication ya está conectado por fuente en
`inventory_swap/positions.rs`, World `inventory_moves.rs` y su constructor
en `player_items/publication.rs`. App retiene el participante Stats seleccionado;
se rechazó y retiró una factory Stats añadida al rol Inventory y las referencias
App al catálogo/config completos. El constructor Stats existente es inerte;
su nuevo reborrow conserva referencias y las consultas siguen al final después
de Player VALUES. La vista de slots de bolsa usa el accessor canónico original.
Tres casos escritos cubren dedup y orden bolsa → Player → Stats, mapa actual
sin gear/entrada vacía y ausencia de reconstrucción desde fixture sin handle.
Export normal conectado, revisión por fuente favorable; no hay ejecución ni
captura nueva. Child/offhand y el executor swap completo siguen pendientes;
la liberación completa de botín se trabaja como dependencia compartida con Save.
El contraste del siguiente mapa ubica AutoUnequipOffhand en
Player.cpp:24600–24645 del SHA target: cuando CanStore falla, C++ mueve el item
fuera del inventario y lo envía por correo en una transacción. El executor Rust
actual de `inventory_moves/item_mutations.rs` conserva el offhand si StorageMove
no lo mueve y registra `needs_mail_fallback: false` cuando sí se trasladó.
Ese contraste queda pendiente de F6; F5 no introduce silenciosamente correo ni
declara paridad por trasladar el executor. Child también conserva la búsqueda
nativa del snapshot completo y sus consultas tardías de desplazamiento.

El mapa source-only de child/offhand identifica StorageMove completo en
`handlers/character/items.rs:171–635` como dependencia, incluso con
`QuestChecks::None`: conserva conteos frescos y obtain-spells async. El siguiente
corte asignado es la relocalización committed completa de
`session/player_items/persistence.rs:15–129` hacia
`inventory_swap/relocation.rs`, con participantes seleccionados y una operación
Core finita final. El corte ya tiene fachada World, módulo App y hoja Core
conectados por fuente, conservando escrituras independientes, snapshots completos,
dos recorridos de children en orden nativo y GUID tardío; sus callers mantienen
el rollback temporal del redirect. Cinco pruebas escritas cubren posiciones
iguales/destino ocupado, owner stale con reemplazo del mismo GUID, traslado y
reversión, bolsa canónica con children y fuente/container ausentes. La salida por
snapshot de objetos ausente sigue después de las escrituras top-level originales;
no se convirtió en un preflight nuevo. Revisión por fuente favorable, sin
ejecución ni aceptación. RawEquip,
StorageMove y child/offhand completos siguen pendientes dentro de F5.

El proveedor completo de persistencia de encantamientos ya tiene acceso
seleccionado en Inventory y hoja Core `inventory/enchantment.rs` montada. La
fachada Hub delega sin capturar datos: primero se clona el item desde el runtime
completo, después se consultan duraciones mediante el accessor canónico original,
y el default vacío permanece en Inventory. Se mantienen recorrido por índice,
primer duration coincidente, flags MAINHAND_ONLY/DO_NOT_SAVE y espacios finales.
Tres pruebas escritas cubren catálogo ausente, override canónico/fallback del item
y rechazo de fixture con owner stale reemplazado por el mismo GUID; comprueban
ausencia de mutación y publicación. Revisión por fuente favorable, sin ejecución.
Este proveedor permite el siguiente traslado completo de rawEquip, todavía pendiente.

El contraste de ActivateToQuest conserva dos diferencias adicionales para F6:
`GameObject.cpp:2218–2265`, SHA target `a5f8da2eb`, consulta en Chest
`Battleground::CanActivateGO` después de los requisitos de quest/loot, mientras
Rust no aplica ese gate; Generic consulta el quest ID del template en C++,
mientras Rust repite HasQuestForGO por objetivos. El inventario de ObjectMgr
para Generic también depende del quest ID del template (`ObjectMgr.cpp:8780+`).
El traslado F5 conserva los cuerpos Rust completos y sus consultas frescas;
no acredita paridad ni introduce esas reparaciones sin el contrato F6.

Quest eligibility de visibilidad también está escrito como módulo privado: nivel,
race/class y CanSeeStart conservan disable, status, recurrence, seasonal, prev-quest
y consultas tardías de level/hide-diff del cuerpo Rust. C++ Player.cpp:14073 y :15033,
SHA a5f8da2ebf5424bf0450ca4e08843ecbf72577bd, no demuestran paridad de ese helper
bounded: CanSeeStart aplica además skill, exclusive, reputation, day/week/month y
expansion. F5 mantiene la representación Rust; diálogo, condiciones y completions
enteros siguen abiertos, sin callbacks World ni prueba ejecutada.
El diálogo completo ya está conectado posteriormente en
`quest/visibility/dialog_status.rs`, reutilizando el participante readonly
QuestEligibility y una sola definición del enum de fuente. Las dos fachadas
World conservan `&self` y el catálogo QuestInfo opcional. Mantiene enders antes
de starters, status fresco con retorno NONE si falta owner, CanTake antes de
los filtros repeatable y condiciones completas antes del status/CanSee/level
de starters. Tres escenarios nuevos de `quest_tests/creature.rs` comprueban OR
de flags, prioridad del owner canónico sobre el fixture y rechazo del fallback
tras reemplazar el owner con el mismo GUID. Se revisaron cuerpos y fixtures por
fuente; no se ejecutaron pruebas, capturas ni aceptación. Este avance sustituye
solo el pendiente de diálogo citado arriba: ActivateToQuest, refresh completo y
CompleteQuest siguen abiertos.
También se escribieron los gates completos de skill y reputation: skill mantiene
conversión y proyección de records actuales; reputation consulta min/max por
separado mediante un lector Core de faction/manager, con race/class tardíos,
catálogo ausente como standing cero y fallback original solo sin handle. La
revisión por fuente no sustituye su aceptación ni el CanTakeQuest completo,
todavía pendiente junto con sus condiciones y consumidores de diálogo.
Exclusive-group ya conserva el gate positivo y catálogo ausente antes del
snapshot actual, recorrido nativo de peers, cooldowns DF/day/week/seasonal y
status/rewarded de la pareja repeatable. Se contrastó por fuente con World/
eligibility:339–414 y C++ Player.cpp:15345–15410; sigue privado, NO VALIDADO,
sin acreditar el coordinador de admisión completo ni sus consumidores.
La evaluación QuestAvailable completa ya está escrita sobre la proyección
seleccionada de PlayerCondition. Conserva el fallback local/global de catálogo,
gate antes de leer owner, recurrence actual y vectores nativos, proyección
íntegra, snapshots tardíos y evaluación de área por jerarquía. Se revisó contra
World/eligibility:117–240, sin reutilizar las reglas distintas de Trainer.
La conexión a CanTake/diálogo y sus pruebas siguen pendientes, NO VALIDADAS.
El cuerpo CanTake completo ya se escribió con recurrence inicial, gates frescos
de exclusive-group/condiciones, cooldowns ordenados y expansion al final. Se
revisó contra World/eligibility:524–768 y el orden C++ Player.cpp:14090–14102;
Timed y demás diferencias representadas no se repararon dentro de F5. Se detectó
en el nuevo factory un argumento consumer-test sin el cfg de su parámetro y se
asignó corregirlo. Conexión de consumidores/diálogo y aceptación siguen pendientes.
El cfg del argumento ya está corregido por fuente. La revisión del consumidor
detectó otro problema: construir RewardCx para CanTake exigía mutable Core,
Inventory, Lifecycle y QuestState y ampliaba consultas World de &self a &mut self.
Ese límite se rechazó: la admisión completa requiere un contexto de solo lectura
y una capacidad Core finita, con proyección de condiciones inerte y lecturas
tardías originales. Se asignó conservar el cuerpo completo y restaurar firmas
de lectura, sin cambiar gameplay ni duplicar autoridad. Esa corrección ya está
conectada por fuente: QuestEligibilityCx toma QuestState/condiciones compartidos,
QuestEligibilityAccess conserva Core privado de solo lectura y World vuelve a
`can_take_quest(&self)`. Los consumidores de status, diálogo, menú y activate
conservan sus firmas de lectura; los handlers de paquetes siguen mutables por
decode/envío. Core y App tienen exports normales. Se corrigió una referencia
obsoleta al consumer-test y la expansión final usa el lector Core seleccionado.
Los casos existentes de quest_6/quest_7 y scenarios_quest_1 se conservaron sin
ejecutar; no hay evidencia de compilación, aceptación ni nueva paridad. Timed
continúa como diferencia explícita de F6. Revisar el cuerpo anterior no aprobaba
el consumidor mutable que fue sustituido.

Consulta compartida de slot (2026-10-03, NO VALIDADO): el ancla original es
`handlers/quest/state.rs:327`; toma un snapshot actual, exige slot menor que
MAX_QUEST_LOG_SIZE y status incomplete/complete/failed. Su proveedor App se
asignó al dueño de objective_progress para reutilizar la consulta existente,
con fallback World-test/no-handle primero y consulta canónica en los demás
casos. Reward reborrowará la capacidad de objetivos y hará esa lectura después
del cálculo de XP, sin adelantar snapshots ni duplicar el filtro en otro estado.

Dependencia FeignDeath de Trainer (2026-10-03, NO VALIDADO): el contrato de
traslado se fijó contra `movement/state.rs:87`: consulta DIED en el owner,
snapshot de auras visibles, colección de slots en su orden de iteración,
RemoveAura completo por slot ignorando su resultado, nueva mutación para limpiar
DIED y retorno true. Un erase de slots no cubre el contrato. List conserva el
clone del store después de NPC admission y antes de resolver trainer; ofertas,
conteo, locale/greeting y reemplazo de interacción permanecen en su fase.
Se asignó escribir el coordinador con esas dependencias finitas pendientes;
no se declara cerrado por existir el helper de publicación.

Revisión de Unique/CanEquip (2026-10-03, NO VALIDADO): el nuevo cálculo de
equipados y gemas conserva las fases de `player_items/equipment.rs:214–328`.
El DTO de candidato para AvgTotal debe conservar además el filtro NonEquip
del accessor `Core/player_items/storage.rs:5`, que produce None y no Some(0).
Ese filtro se asignó; el cuerpo CanEquip completo aún es un placeholder y
debe reemplazarse antes de conectar/aceptar la operación. La revisión manual
de manifest, lock y usos nuevos no detectó otra dependencia ausente; no ejecutó
Cargo metadata ni constituye comprobación del grafo efectivo.

Primer coordinador App TrainerList (2026-10-03, NO VALIDADO): el cuerpo completo
ya está escrito en `trainer_purchase/controller.rs:140–256`; la inspección frente
al original confirma admission NPC, clone/resolución del store, feign death,
ofertas/fallback de precio, conteo, greeting y publicación del rol/paquete en
su orden. Faltan los proveedores de ofertas y retirada completa de auras,
constructor externo de stores seleccionados y fachada World. La referencia
de locale apunta al mismo campo Core sin clone; no es un snapshot propio.
La publicación ya reutiliza publish_trainer_list_like_cpp después de greeting;
el constructor seleccionado y la fachada World también están escritos. El
provider App de ofertas ahora conserva la secuencia ID, filas completas de
spells y skills, conversión de requisito, clasificación battle-pet, precio y
preflight del original. Aún falta definir finish_trainer_offer_projection_like_cpp
en el punto NeedsProjection, además de la retirada completa de FeignDeath.
Estas conexiones no prueban compilación ni cierran la familia; TrainerBuy
continúa pendiente de coordinador completo.

La integración de FeignDeath requiere préstamos secuenciales (decisión de
2026-10-03, todavía sin aceptación): ListCx conservará el participante Aura
mutable y builders inertes, sin retener simultáneamente SpellState ni una
PlayerConditionProjection readonly. NPC presta temporalmente vitals para
admisión; termina ese préstamo antes de clone/resolve TrainerStore y de la
retirada completa de FeignDeath. Después, cada oferta construye vistas readonly
de los owners actuales, conservando sus consultas frescas. También se solapan
vitals y referencias de auras con Stats; los builders las reciben por reborrow,
no por snapshots adelantados. Las vistas terminan antes de publicar interaction
y el paquete. Implementación asignada; esta decisión no acredita que el nuevo
constructor ni sus consumidores estén terminados o compilados.

TrainerList ya entregó esa integración completa por fuente: controller y row
projection App, builders Core, vistas temporales desde Aura, inputs de
PlayerCondition y constructor World. NPC → clone/resolve store → fullFeign →
ofertas frescas → count/log → greeting → reemplazo de interacción → envío
conservan el orden. Buy conserva su consulta standalone de oferta. Dos pruebas
nuevas en `handlers/trainer/tests/admission.rs` fijan la oferta Unavailable antes
de Feign y Available después, con bytes AuraUpdate antes de TrainerList, y el
rechazo de NPC sin alterar aura/provenance ni enviar paquetes. El caso existente
de mapping ausente conserva resolución antes de Feign. Revisión por fuente
favorable, sin ejecutar; no hay evidencia de compilación ni aceptación. El mapa
completo de TrainerBuy es la siguiente responsabilidad asignada.

El primer corte completo de Buy ya está conectado por fuente:
`trainer_purchase/buy_admission.rs` conserva decode/log → NPC → fullFeign →
provenance/store/member → rechazos exactos y devuelve una request owned. El
contexto seleccionado no retiene CoreAcquisition mutable ni ConditionProjection;
constructor World y export normal están escritos. El lookup se comparte con
Buy existente y los dos reads del log de mismatch siguen separados. Dos nuevos
casos en `handlers/trainer/tests/failures.rs` cubren parse-failure sin cambios
y spell ausente con AuraUpdate antes de TrainerBuyFailed reason 0, sin cargo.
Se revisaron por fuente; no se ejecutaron. World conserva el cuerpo posterior
de Save/dinero. Full Save, saga BattlePet, cierre de Buy y aceptación siguen
pendientes; sustituir el preflush completo por capture/persist/ACK App no sería
un traslado fiel. Se asignó el mapa acotado de ese coordinador de guardado.

Proveedor de precio de trainer (2026-10-03, NO VALIDADO): el nuevo método
NpcInteractionAccess de faction_reactions.rs conserva los retornos Neutral por
store/faction ausentes, identidad antes del guard, rank dentro del guard de
ReputationMgr y fallback Core test-fixtures/no-handle. La fachada Hub anterior
ya delega a ese único cuerpo. Este alcance difiere del cálculo de rank de
elegibilidad de inventario, cuyo original libera el guard antes de convertir
standing; los dos contratos no deben unificarse alterando sus fases.

Cierre de consumidores de condiciones (2026-10-03, NO VALIDADO): el constructor
compartido World ya suministra reputación, battleground e in-combat requeridos
por valoración, y spell-click lo reutiliza. Sin embargo el holder, as_context y
la proyección antiguos siguen completos en World y mantienen consumidores en
trainer, limit-category y meets-player-condition. Su retiro forma parte del
mismo traslado; disponer del constructor App no demuestra una autoridad única
ni autoriza cerrar condiciones. Se notificaron esos consumidores al dueño.

Retiro del cálculo duplicado (2026-10-03, NO VALIDADO): World ahora conserva
un holder fino que delega project_like_cpp y condition_context_like_cpp a App.
La conversión prestada es pública y separa el préstamo temporal del contexto
de las referencias de salida. AvgTotal y su helper exclusivo se retiraron de
World; el proyector App calcula AvgTotal antes de AvgEquipped. La revisión del
delegado equipado detectó una lectura rápida de runtime-item que sustituía la
proyección completa original; el delegado ya usa el provider completo compartido
resolved_player_inventory_item_object_with_access_like_cpp por cada slot.
CanUse mantiene por fuente consultas de skill-records y su proyección a valores,
requisitos de spell, reputación, efectos y especialización del snapshot inicial.
CanUse World/persistence_load ya delega el cuerpo completo en el export App,
manteniendo not_loading y sin proyectar condiciones al construir su vista.
El handoff nativo del implementador confirma el checkpoint de esta unidad
NO VALIDADO. World/equipment ya delega CanEquip y Unique a las entradas
seleccionadas de App; la revisión conserva slot, swap, not_loading, combate,
dual-wield y titan-grip, además de las consultas previas de PlanEquip. La revisión
posterior confirma ambos exports normales de esas entradas en App/lib.rs.
CanStore y el macro Inventory permanecen abiertos; no hay aceptación.

Conexión del guardado canónico (2026-10-03, NO VALIDADO): World ya llama a
`save_canonical_player_like_cpp` después de adquirir sus fences. La revisión
confirma captura única, ACK canónico y tutorials dentro de App, retorno del
indicador de Registry, liberación del owner y publicación Registry después.
El camino World-test/no-handle mantiene su receipt fixture anterior. Los
retornos de snapshot no disponible liberan money lock y admisión antes del
drenaje. La coordinación de timer, transferencia, loot, reconciliación y
drenaje sigue World; falta su traslado completo, mounts y aceptación.

Contrato de drenaje verificado (2026-10-03, NO VALIDADO): el cuerpo actual de
World/session/quest/objectives.rs:662–748 convierte currency/faction a object_id
con fallback i32::MAX antes del primer await. Cada evento Currency ejecuta
update_currency, HAVE y OBTAIN secuencialmente; Reputation ejecuta MIN, MAX e
INCREASE. Se extrae un solo evento antes de cada secuencia y finish se llama
únicamente al agotar normalmente la cola. La persistencia y sincronización
Registry de cada update no se agrupan al final. El traslado completo a App
está en implementación; esta inspección fija el contrato sin ejecutar pruebas.

Pruebas de receipt escritas (2026-10-03, NO EJECUTADAS):
World/unit_tests/session/tests/lifecycle_persistence/save_interleaving.rs añade
full_save_stale_core_receipt_does_not_acknowledge_retired_handle y
full_save_receipt_preserves_tutorials_changed_after_capture. La primera retira
el owner durante la persistencia con try_lock y comprueba que la fila New no
se limpia. La segunda usa captura y receipt canónicos, persistencia App y ACK
con un tutorial modificado después de la captura; exige conservar valor y dirty.
RecordingPort registra la request antes del hook y devuelve sus committed groups.
Los casos cubren riesgos distintos; su existencia no acredita PASS ni DB real.
La revisión solicita además reemplazo del owner con el mismo GUID y otra
incarnación/generación: retirar y observar el Player viejo no demuestra por sí
solo que un receipt antiguo deje intactas las filas del nuevo owner.
Ese caso ya está escrito como extensión del test de owner retirado: instala
otro Player con igual GUID y generación distinta durante el hook, y conserva
dirty tanto en la fila compartida como en una exclusiva del nuevo owner.
También está escrito `full_save_receipt_intersects_committed_groups_before_acknowledging_rows_and_tutorials`:
captura un receipt real y combina grupos esperados no confirmados y un grupo
confirmado ausente de la solicitud; exige conservar filas y tutorial dirty.
La revisión corrigió un helper inexistente y la expectativa del retorno de ACK
(indica necesidad de Registry sync, no mera presencia del receipt). Ambos casos
siguen NO EJECUTADOS; no prueban durabilidad con DB/restart.

Primer cuerpo CanEquip completo (2026-10-03, NO VALIDADO): el placeholder ya
está reemplazado por la operación App. La revisión detectó que omitía CanUnequip
cuando no había offhand y reutilizaba el runtime offhand de un snapshot anterior
para CanStore; el original consulta CanUnequip siempre y relee posición/runtime
en su wrapper de almacenamiento. Se asignó restaurar esas consultas, además de
corregir un argumento de referencia y el nombre STUNNED. El export Core del
owner mutable de guardado ya está presente. No hay compilación de este cuerpo
ni cierre del proveedor compartido de almacenamiento.

Revisión de relecturas de offhand (2026-10-03, NO VALIDADO): CanUnequip ya se
consulta siempre y su argumento proto conserva la referencia. La relectura de
CanStore aún debe mantener la guarda del snapshot offhand previo y usar el
provider de posición original; los accesos rápidos a un solo item no sustituyen
la proyección runtime/clones de los helpers antiguos. Mainhand requiere la
misma conservación. CanUnequip general pertenece a Inventory y debe tener un
solo cuerpo seleccionado reutilizado por App, conservando bag/slot/swap para
sus otros consumidores, en lugar de otro algoritmo exclusivo de offhand.

CanUnequip general conectado (2026-10-03, NO VALIDADO): Inventory/equipment
ya conserva ese cuerpo único en can_unequip_inventory_item_at_with_access_like_cpp;
la entrada Hub delega y App pasa BAG_0/OFFHAND/swap=false. El contraste con el
cuerpo de HEAD confirma posición y retorno temprano, snapshot, charm, snapshot
BG con consulta Map condicionada, combate y selector final, conservando también
los argumentos generales para bolsas y swaps. El provider de reputación de
Inventory convierte standing a rank después de liberar el acceso al Player;
no se fusiona con el provider de precio de trainer, cuyo orden es distinto.
El implementador de valoración continúa activo; esta revisión de fuente no
establece compilación, pruebas ni cierre de AvgTotal/condiciones.

Conexiones World TrainerList y slot (2026-10-03, NO VALIDADO): TrainerList ya
delega su operación a App y el catálogo seleccionado tiene constructor público.
En la primera conexión faltaban ofertas y FeignDeath; el provider de ofertas
ya está escrito, pero faltan su proyección final y FeignDeath. La entrega no
está cerrada por esa fachada. El worker quedó idle tras escribirla y se
reanudó con la unidad acotada de ofertas. Los exports de slot/MAX ya existen y
World conserva su nombre de constante mediante reexport de la única autoridad
App. El mount de las dos funciones de guardado ya está escrito en App/lib.rs;
sus definiciones existen en player_save.rs, frente a la lectura desfasada que
lo dejó pendiente.

Primer proveedor compartido de almacenamiento (2026-10-03, NO VALIDADO): el
cuerpo está escrito en inventory_valuation/direct_storage.rs y World ya tiene
fachada para el plan completo. Conserva por fuente posiciones vacadas, overlays,
templates, referencias de almacenamiento y consulta tardía de limit-category.
Faltan su mount privado y correcciones de nombres de métodos del snapshot;
se asignaron a sus dueños sin introducir callbacks de vuelta a World.
El helper de dinero de Reward ya propaga ausencia de saldo al abort y conserva
la aceptación/rechazo entero del delta de loot_money_durable_outcome original.
No se reclama compilación ni aceptación de ninguno de esos proveedores.

Conexión del plan compartido (2026-10-03, NO VALIDADO): direct_storage ya tiene
su mount privado y los nombres originales de store/register bag del snapshot.
CanEquip restaura la guarda del offhand previo y la consulta fresca de posición
antes de almacenar; Inventory/quest_reward ya define el provider
resolved_player_inventory_item_object_with_access_like_cpp mediante la
proyección completa original y CanEquip lo llama. La hoja direct_storage usa
imports explícitos, sin depender de imports del módulo padre. STUNNED también
está corregido. Estas correcciones de fuente no sustituyen las pruebas pendientes ni demuestran
cierre de los consumidores de almacenamiento o recompensas.

Revisión de transición de reposo (2026-10-03, NO VALIDADO): Core XP ya instala
el reposo canónico mediante una única with_owned_player_mut/mutate_rest_state;
la sustitución del snapshot queda limitada al fixture sin handle. El participante
Rest acotado ya está escrito mediante entradas estáticas con referencias
seleccionadas: las dos fachadas de test no construyen QuestReward/Stats ni llaman
al constructor privado de XP. FixtureTake conserva victim vacío, bonus, premio,
modifier, pérdida y setter incondicional también con handle canónico; mantiene
la normalización cuando el premio entero es cero. La inspección contrasta esa
secuencia con el cuerpo original de HEAD. Sigue pendiente aceptación de las
ramas canónica y fixture; no se han ejecutado compilación ni pruebas.

Revisión de condiciones de trainer (2026-10-03, NO VALIDADO): el nuevo cuerpo
App conserva store, objeto jugador, proyección completa, snapshots Unit/Player
y evaluación por condición con contexto vivo y bandera Unsupported. Se mantiene
el predicado de igualdad de área del trainer, distinto al IsInArea de spell-click.
Se detectó un tipo de store referenciado desde wow-conditions sin reexport;
se asignó usar el owner wow-data. La proyección tardía y los coordinadores List/Buy
siguen pendientes; el helper no demuestra su operación completa.

### F6 — retirada de duplicados, pista de comportamiento

Contraste de la reentrada condiciones/valoración (2026-10-03, fuente C++):
`Player.cpp:28732–28747::GetItemLimitCategoryQuantity`, SHA
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`, evalúa las condiciones por fila.
`ConditionMgr.cpp:3206–3216` lee los campos canónicos almacenados
`m_playerData->AvgItemLevel[0/1]`; no llama a los calculadores de promedio.
`Player.cpp:28803–28877` actualiza esos campos después de los cálculos completos.
La reentrada del holder Rust no queda acreditada como paridad por ese código.
F6 necesita localizar la autoridad y todos los writers/readers de esos valores,
con actualización/publicación y pruebas de la condición del límite; añadir un
cache de sesión o quitar la lectura del promedio no satisface ese contrato.
El descriptor Rust `AvgItemLevel` por sí solo no demuestra almacenamiento efectivo.

Contrato de extracción RewardQuest (2026-10-03, fuente Rust NO VALIDADA):
`World/handlers/quest/rewards.rs:563–785` todavía posee el coordinador completo.
Su cierre incluye removals, grants, moneda/skills, plan de dinero, títulos/mail/
lockout, proyección de XP/slot/status, COMMIT, settlement, objetivos/Registry,
slot, notificación GameEvent, paquetes, reputación/spells y XP antes de liberar
el flag de teleport diferido. La rama de settlement fallido tras COMMIT hace
kick y retorna sin liberar ese flag; F5 debe conservarla y F6 contrastarla,
sin introducir una reparación RAII dentro del movimiento. Los helpers existentes
de QuestRewardCx no demuestran ese flujo completo ni el drain que lo consume.

Effects de Reward escritos (2026-10-03, NO VALIDADO): la hoja privada effects
conserva por fuente los registros de skill/title/mail condicionados al consumidor
World-test y la mutación canónica de talent points antes del fallback sin handle.
La revisión detectó que el registro de spells leía can_delay directamente del
fixture, mientras el original consulta el snapshot de teleport canónico por cada
registro. La hoja ya consulta el participante QuestRewardPlayerAccess por cada
spell, delegando al mismo snapshot canónico con fallback sin handle en Core
movement/transfer. No cachea un bool previo ni construye Hub. Los cuerpos originales
siguen presentes y deben delegar una vez cerrado el conjunto; no hay aceptación.

Retirada de requisitos escrita (2026-10-03, NO VALIDADO): App/reward/removals
ya contiene el cuerpo completo de planificación de objetos y monedas, con los
rollback presentes en el original, item-drop después de objetivos, filas currency
vacías dentro del plan QuestTurnIn y aplicación de objetos antes de los paquetes
de pérdida de monedas. Los casts comprobados conservan el salto de publicación
si cantidad o pérdida no caben en i32. Inventory/turnins ya aplica el cuerpo
seleccionado completo: updates y sus paquetes dentro del loop, las dos mutaciones
de delete, valores al final y retorno del indicador para la fase App de stats.
Los proveedores currency mantienen snapshots frescos y Some(0) cuando falta una
moneda; el coordinador ya pasa las referencias fixture de stats. La revisión
posterior confirma que World/handlers/quest/rewards delega la retirada completa
mediante QuestRewardCx y once referencias fixture de stats; World/handlers/
character/items::apply_item_turnin_changes delega la aplicación en Inventory,
conservando la publicación de stats después del resultado. Quedan los demás
consumidores del macro Reward y su aceptación; estas conexiones no prueban su cierre.

Revisión de integración pendiente (2026-10-03, NO VALIDADO): objectives App
ya contiene las ramas de dinero, moneda y reputación, además de storing-value y
storing-flag. La lectura detecta cuatro constantes referidas en wow_constants::quest
que siguen siendo locales World/session/mod.rs: currency 4, min-reputation 6,
max-reputation 7 y money 8. Su autoridad y consumidores requieren cierre antes de
aceptación. El snapshot de adquisición de entrenadores también usa dos providers
Core que acceden a self.core.fixtures, inexistente en ese owner; deben recibir
referencias fixture prestadas seleccionadas y conservar los gates NoHandle y de
completitud originales, sin añadir una segunda autoridad. Ambos hallazgos están
asignados a los implementadores; no se declara compilación ni corrección ejecutada.

Revisión de grants Reward (2026-10-03, NO VALIDADO): App/reward/grants contiene
los loops completos de entrega fija, elegida y package, con las mismas colecciones
primary/fallback y selección previa al await que World/rewards/items. La hoja
item_storage ya usa el provider completo para existing-stack, como el original
World/Inventory/storage::resolved_inventory_item_object_like_cpp, que proyecta
todo el runtime. El getter rápido usado por EquipmentSet permanece intacto.
Player.cpp:14582–14623::RewardQuestPackage, SHA C++ a5f8da2ebf5424bf0450ca4e08843ecbf72577bd,
continúa el loop ante CanStoreNewItem rechazado y genera random properties; el
Rust original devuelve false/publica error y representa propiedades cero. Son
diferencias F6, no reparaciones implícitas de F5. El gate de plantilla elegida
antes de package queda contrastado con Player.cpp:14713–14731.
World/rewards/items ya delega fixed/chosen/package
mediante un composer privado de QuestRewardCx, con referencias obligatorias
prestadas para las condiciones de almacenamiento y vitals. La planificación usa
NULL_BAG/NULL_SLOT, source None, swap false y overlays/vacated vacíos como antes.
Los exports App de esas referencias fixture ya están conectados. El coordinador
World/rewards/controller ya construye los participantes y delega al coordinador
App completo. La revisión confirma préstamos disjuntos de XP/vitals y referencias
inertes de planificación, con una única referencia mutable de reputación. La
publicación del quest-log ya está conectada al cuerpo completo descrito abajo;
se revisa el cierre de los consumidores restantes. Esta conexión no acredita
aceptación del macro Reward.

Contraste de reputación Reward (2026-10-03, NO VALIDADO): el cuerpo World/
rewards.rs::record_represented_quest_reward_reputation_like_cpp también modifica
ReputationMgr y publica standing; mover solo sus registros fixture dejaría fuera
el comportamiento normal. Conserva clones de catálogos antes del loop, ganancias,
RAF, consulta actual del rank y construcción del paquete dentro de la mutación
del manager, con envío después del guard. Su fachada World ya delega el cuerpo.
C++ Player.cpp:6450–6500::RewardReputation, SHA
a5f8da2ebf5424bf0450ca4e08843ecbf72577bd, aplica rank-cap sobre rep base positiva
antes de CalculateReputationGain (6321); Rust lo consulta después de ganancias y
RAF, condicionado por la positividad del resultado. Esa diferencia de fase es F6,
no una reparación oculta en el traslado F5. La representación Rust sin stores
tampoco prueba los efectos reales de C++ ante esas dependencias ausentes.
El cuerpo completo de reputación ya está escrito en App/reward/reputation y usa
el mismo cálculo de quest-level, aura gain, gray y RAF, además de la mutación y
publicación ordenadas. Su integración requiere resolver el préstamo compartido:
planning y objetivos no pueden retener un &reputation fixture mientras Reward
retiene un &mut del mismo estado. Planning ya recibe la lectura temporal por
argumento y objetivos toma prestado el mismo holder mutable de Reward. El getter
readonly y el reborrow están limitados a crate::quest para ese consumidor hermano.
No se añaden clones, mirrors ni RefCell. No se declara cerrado el coordinador
ni ejecutada su aceptación.

Correcciones posteriores revisadas por fuente (2026-10-03, NO VALIDADO): las
cuatro constantes de objetivos ya están definidas en wow_constants::quest.
Core/spell_acquisition recibe referencias prestadas obligatorias de completitud,
ocupación y tombstones; conserva canonical primero, NoHandle y Some(None) sin
fallback. App/projection pasa esas referencias y distingue consumer_test según
cfg. Reward/item_storage ya usa el provider de runtime completo para existing-stack.
No se ejecutó validación; la proyección completa de ofertas está escrita, como
se detalla abajo. Siguen pendientes sus consumidores, los restantes consumidores
Reward y el cierre de completion/Save.

Contrato de completion revisado (2026-10-03, NO VALIDADO): World/handlers/quest/
state.rs::complete_represented_quest_like_cpp ejecuta invalidación, transición,
registro, refresh de gameobjects/spell-click y publicación Registry. El kernel de
estado App no sustituye ese sufijo. AfterObjective vuelve a consultar snapshot,
status y reglas; tras completar devuelve true incluso si tracking Reward falla,
pero marca auto-reward y drena recursivamente con Box::pin solamente si rewarded
es true. Se conserva ese contrato F5. C++ Player.cpp:14496::CompleteQuest, SHA
a5f8da2ebf5424bf0450ca4e08843ecbf72577bd, incluye SetQuestStatus, tracking Reward,
flag del slot y tracker; SetQuestStatus (15557) contiene SendQuestUpdate y scripts.
Los registros Rust que declaran efectos unrepresented no prueban paridad F6.
App/visibility ya contiene los filtros
HasQuestForGameObject/IsForQuests, pero su presencia no cierra los consumidores
World/quest_interaction ni el refresh completo.
La revisión del contexto de objetivos detectó un préstamo fixture de level
redundante con el mutable retenido por QuestRewardCx; ya se lee el mismo
player_level en la fase Registry, sin copiarlo ni crear otra autoridad. Su export
fixture de Registry ya está conectado en App/lib.rs. El sufijo Reward de
quest-log requiere trasladar también quest_log_create_entries: debe leer entradas
actuales después de COMMIT/settle, conservar el límite 25 y los masks de todos los
campos del slot antes del UpdateObject; no basta una entrada vacía o precapturada.
App/quest_log ya contiene ese cuerpo completo: snapshot inicial, consulta fresca
por slot, rechazo de duplicados, 24 contadores/flags y publicación con GUID/map ID
en la misma fase. Mantiene el cast del contador negativo del baseline Rust. El
sufijo Reward ya lo invoca; las tres fachadas World compartidas y sus exports
App también están conectados, conservando cfg!(test) del consumidor.
C++ Player.cpp:15963–16002::SetQuestSlot/Counter/State/EndTime, SHA
a5f8da2ebf5424bf0450ca4e08843ecbf72577bd, confirma los campos del slot; no prueba
los bytes ni el cast negativo Rust sin aceptación específica.

Dependencias del refresh contrastadas (2026-10-03, NO VALIDADO): World/gameobject_query
consulta GUIDs visibles, entrada canónica y una copia del use-state por objeto;
evalúa HasQuest e IsForQuests por separado antes de DynamicFlags y lee el map ID
en la fase de publicación. Después ejecuta spell-click con su propio snapshot;
este exige ambos stores antes del scan y solo publica criaturas con SPELLCLICK y
algún click condicionado. DynamicFlags depende también de ActivateToQuest, el
estado por viewer, GM y condiciones; IsForQuests o flags crudos no lo sustituyen.
ActivateToQuest incluye el diálogo de questgiver (handlers/quest/eligibility) y
loot de misión específico del jugador (session/loot/operations), no únicamente
loot genérico del catálogo. Ese cierre está asignado como módulos privados de
aplicación; no autoriza callbacks World ni precalcular resultados de esos gates.
El contrato asignado de loot de misión conserva el OR de objetivo directo, addon QuestLogItemID
y item-drop, con snapshots separados por rama y counts actuales solo para cada
drop coincidente. C++ Player.cpp:16343::HasQuestForItem y :16386::GetQuestObjectiveForItem,
SHA a5f8da2ebf5424bf0450ca4e08843ecbf72577bd, aplican además el gate raid/dificultad
con excepción de battleground, ausente del cuerpo Rust World actual. Es una
diferencia F6; el traslado no debe añadirla silenciosamente. GetItemCount(item,true)
también exige contrastar el alcance de banco/trade antes de acreditar paridad.
LootMgr.cpp:208::HaveQuestLootForPlayer delega al template específico del jugador.

Revisión del traslado de loot de misión (2026-10-03, NO VALIDADO):
Application/quest/loot_requirements.rs ya contiene la operación completa y sus
predicados privados. Conserva conversión del item antes del snapshot de objetivos,
store antes del snapshot de item-drop, iteración nativa y lectura completa de counts
por drop coincidente. World/session/loot/operations.rs ya delega con roles
seleccionados y cfg!(test) del consumidor. quest/mod.rs y Application/lib.rs ya
exportan la operación; se verificó la integración del reexport solicitado al
propietario de ese archivo. Este avance no cierra ActivateToQuest, diálogo ni refresh de visibilidad,
ni acredita ejecución, bytes o paridad con los gates C++ pendientes.

Proyección completa de ofertas escrita (2026-10-03, NO VALIDADO): App/trainer_purchase/
projection contiene finish, wrapper resolution, snapshot fresco, metadata y el
planificador existente; capacity solo se consulta tras un plan Deterministic.
La comparación con World/spell_acquisition/adapter y World/profession conserva
wrapper antes del snapshot, metadata después, fallback cast/craft tipado y la
secuencia skill-lines, loaded, skill-records actuales, análisis y requested-plan.
Los stores seleccionados son referencias prestadas inertes. El provider Spell
seleccionado mantiene el cuerpo anterior del wrapper; su fachada Hub delega al
mismo cuerpo. El constructor World ya conecta los stores y refs prestadas y
offer delega a App. El implementador confirma el cierre por fuente sin ejecutar
checks. Quedan Feign/Buy del macro Trainer y aceptación; esta unidad no los sustituye.

Revisión del snapshot spell-click (2026-10-03, NO VALIDADO): el cuerpo Domain
ahora recibe QuestObjectiveAccess. Comparado con el cuerpo Hub de HEAD, conserva
los rechazos de GUID, el fallo por lock envenenado, la búsqueda de mapa con
instance cero y la selección creature/pet. La construcción mantiene la copia de
PhaseShift, todos los campos de criatura y la preferencia pet-owner antes del
control-owner. La fachada World todavía usa su split Hub y el filtro completo
permanece pendiente de la proyección tardía; este traslado no acredita el flujo
de publicación completo ni su paridad con C++.

Contraste TrainerList (2026-10-03, inspección sin aceptación):
`NPCHandler.cpp:113–129::SendTrainerList`, SHA
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`, reinicia y establece InteractionData
antes de `Trainer::SendSpells`. El Rust representado evalúa primero las ofertas
y establece el rol inmediatamente antes del envío, ahora en AppTrainerListCx.
Además, Trainer.cpp:185–225::GetSpellState devuelve Known antes del filtro
race/class; el preflight Rust representado evalúa class/race y condiciones antes
de directly_known. F5 conserva esa diferencia de orden, cuya resolución de
paridad queda en F6 y no se oculta en la extracción. La proyección NeedsProjection
debe volver a consultar filas completas, slots, traits, overrides y tombstones
en el orden de spell_acquisition/adapter.rs:155–322; reutilizar las filas previas
del offer eliminaría consultas originales. No se ha ejecutado aceptación.
F5 conserva ese orden Rust; el helper de publicación no demuestra el flujo
completo ni la equivalencia de orden C++. `Trainer.cpp:185–225::GetSpellState`
consulta spell conocido, clase/raza, skill, habilidades, nivel y efectos LearnSpell;
la extracción debe conservar también las consultas por fila del flujo Rust.
La equivalencia observable y cualquier reparación pertenecen al contrato F6.

Contraste de valoración de equipo para la proyección de condiciones (inspección
2026-10-03, sin aceptación): `Player.cpp:28803–28877`
(`UpdateAverageItemLevelTotal`/`UpdateAverageItemLevelEquipped`) y
`Item.cpp:1891–1940::GetItemLevel`, SHA `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`.
No son el método alternativo `GetAverageItemLevel` de Player.cpp:27909.
C++ recorre `ForEachItem(Everywhere)` y, para equipo no equipado, comprueba
CanEquip antes de GetItemLevel; acumula en float. El Rust actual recorre
snapshots representados directos/anidados, calcula el nivel antes de sus gates
CanUse/Unique/CanEquip y suma enteros con saturación. Su override de nivel debug
tampoco constituye prueba del GetItemLevel C++. F5 preserva esos órdenes y
operaciones; F6 requiere contrastar el consumidor completo antes de acreditar
o reparar las diferencias. Los promedios no se pueden precapturar en World
para sustituir la operación App y sus consultas originales.

Contraste ItemSet de continuación (2026-10-03, 18:38 UTC, inspección sin
aceptación): `Entities/Item/Item.cpp:57–144` (`AddItemsSetItem`) y `:146–196`
(`RemoveItemsSetItem`), SHA `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`, insertan
o retiran el item y sus bonuses y llaman a `Player::ApplyEquipSpell`. En la
retirada, esa llamada precede a borrar el bonus. El proveedor Rust trasladado
en `wow-world-inventory/src/item_sets.rs` conserva su contrato anterior de
mutación y devolución de eventos; ese evento no demuestra por sí mismo la
ejecución de ApplyEquipSpell ni su orden observable. F6 debe contrastar los
consumidores completos y sus efectos, incluidos los de equipamiento inicial,
antes de acreditar o reparar esa diferencia. F5 no introduce esa reparación.

La revisión F5 conserva dos diferencias que no puede resolver mediante un
traslado estructural: assign-equipment-set sigue activo en Rust frente a
STATUS_UNHANDLED/Handle_NULL en Opcodes.cpp:170; la validación Rust de profesiones
0..=11 no tiene ese límite explícito en World.cpp:1135. Se corrigieron sus
comentarios contra `a5f8da2eb` sin cambiar las reglas ni presentar esas
diferencias como divergencias aprobadas. El campo C++ compartido CharacterPoints
se verifica en Player.h:1848–1849/Player.cpp:2359 y los dos slots en
UpdateFields.h:755; no son evidencia de la preparación/durabilidad asíncrona.

La inspección completa de `CharacterHandler.cpp:1953–2010`
(`HandleUseEquipmentSet`, mismo SHA) conserva otro límite existente:
`session/player_items/equipment_sets.rs::use_represented_equipment_set_like_cpp`
usa inventario directo y backpack libre, sin las decisiones C++ `CanStoreItem`,
`CanUnequipItem`, `CanEquipItem` ni el error de equipamiento de la rama de
almacenamiento fallido. El traslado F5 debe conservar esa ruta actual y el
resultado final; corregir su alcance exige el contrato completo de inventario
y evidencia F6. Save se contrasta por separado contra
`CharacterHandler.cpp:1860::HandleEquipmentSetSave` y
`Player.cpp:26376::SetEquipmentSet` antes de diseñar su contexto acotado.
Save conserva además las omisiones Rust de validación ante catálogos ausentes
y de `ScalingClassRestricted` (`CharacterHandler.cpp:1932`); el contexto no
convierte esas diferencias en validaciones nuevas durante el traslado.

La proyección de quests conserva también la tabla Rust de XP sin catálogo y
el índice limitado a nueve. En `a5f8da2eb`,
`Quests/QuestDef.cpp:387–410::Quest::XPValue` devuelve cero sin jugador, sin fila
de XP o con dificultad mayor o igual a diez; su redondeo está en `:714`.
`Player.h:1491–1495::GetQuestLevel` resuelve el nivel escalado del jugador.
Estas anclas no convierten el fallback Rust en paridad: F5 conserva su cuerpo
y sus lecturas condicionales, y F6 mantiene pendiente el contrato de reparación.

La preparación de la familia Instances, revisada en fuente el 2026-10-03 a las
13:52 UTC, conserva otros contratos pendientes contra `a5f8da2eb`:
`SetSavedInstanceExtend` llega en Rust por la declaración de
`SetLootSpecialization` y la forma de nueve bytes del alias `0xBADD`
(`handlers/loot/handlers.rs`); C++ tiene el registro propio en
`Opcodes.cpp:910` y `CalendarHandler.cpp:532`. `SetDifficultyId` y
`ToggleDifficulty` están activos en Rust, mientras C++ los registra
STATUS_UNHANDLED/Handle_NULL en `Opcodes.cpp:889/973`. La respuesta al pending
bind lo consume antes de confirmar en Rust; su rechazo solo incrementa una
fixture bajo cfg(test). C++ ejecuta confirmación o `RepopAtGraveyard` y después
limpia el pending (`MiscHandler.cpp:1061–1076`). El productor productivo del
pending y la autoridad del completed-mask representado aún requieren cierre.
F5 debe preservar esas rutas y el orden de mutación del manager antes del
commit, sin introducir una reparación de protocolo, rollback o gameplay
durante su traslado. Esta revisión no ejecuta pruebas ni demuestra paridad.

Por dominio, retirar la duplicidad `represented_*`/canónica y resolver
`session/legacy_runtime` y el `map_manager` legado. Elegir por la operación completa y
evidencia versionada C++/capturas, no por el nombre de la ruta. Conservar una sola autoridad,
fences y publicación; tests de fallo, capturas y QA live cuando corresponda.
La autorización de runtime/QA heredada de #1241 conserva sus blancos y restricciones;
no autoriza DROP, TRUNCATE, restauración de datos ni exposición de secretos.

### Cierre de la continuación

Cerrar #1263 solo tras F4–F6 y su aceptación. Actualizar este documento y las casillas de
#1263; enlazar el resultado desde #1241 si corresponde, sin volver a cerrarla ni atribuirle
un cierre técnico anterior. Conservar el gate #584 → #583 → #153: esta entrega no cierra
automáticamente la arquitectura global ni el port.

## 7. Rama aparcada

`584-map-manager-domain` @ `5b443512` (WIP de #1233) queda aparcada como referencia; no se fusiona.
Inventario de solo lectura de sus 231 commits (`e786ece1..5b443512`, 2026-09-30), clasificados por
heurística sobre numstat:

| clase | n | wow-world/src | wow-world/tests | otros crates |
|---|---:|---:|---:|---:|
| solo docs | 58 | 0 | 0 | 0 |
| wip (`abfd8aac`, `152a5b9b`) | 2 | −30 733 | +32 750 | **+86 376** |
| candidato a movimiento | 4 | −35 | +25 | +31 |
| crecimiento | 18 | +1 695 | +872 | +300 |
| mixto | 149 | +7 844 | −53 | −111 |

- Los checkpoints WIP copiaron más de lo que movieron: `abfd8aac` quita ~30,7k líneas de
  `wow-world/src` y añade ~29,1k a wow-map, ~15,1k a wow-entities y ~9,3k a world-server. Es
  también el único commit que retira campos de `WorldSession` (`session/state.rs` +142/−452).
- 223 de los 231 commits tocan ficheros que `abfd8aac` reescribe; solo `0b4bd5a3` (−3 líneas) es
  independiente.
- **Decisión:** sin rescate commit a commit. F2 y F3 rehacen los movimientos sobre `3.4.3` limpio
  con codemods, usando el mapeo de ficheros de `abfd8aac` solo como guía.

Evidencia (local del host de desarrollo, fuera de cualquier caché de build):
`/home/server/rustycore-world-refactor-parked-evidence/` — logs de la puerta, manifiestos de
validación y `salvage-inventory/commits.tsv`. El worktree aparcado conserva sus arreglos de puerta
sin commitear; su caché `target` de 63 GB se borró con aprobación del usuario.

Pendientes heredados, a reevaluar solo si se rescata la pieza afectada:

- `handlers/loot/request_cache.rs:65-80`: `next_represented_loot_object_guid_like_cpp` separa
  `cfg(test)` / `cfg(not(test))`; con `test-fixtures` pero sin `test` los brazos de fixture usan
  asignación solo canónica.
- Hallazgos de la puerta en `5b443512`: 188 entradas obsoletas del baseline de campos, crecimiento
  del ratchet de hotspots y arista `wow-world -> wow-spell` sin dueño.

## 8. Estado contrastado — 2026-10-02

Base integrada observada para P4a: `1f8a7c800d56ff3247784148dc94b0dea1c27586` (PR #1264).
Worktree P3: candidato aceptado `002ff5e462ba2c5c948934a270e67b137b456fcf`, limpio durante
final. La consulta inicial no revalidó PRs históricos. #1263 registra la publicación e
integración posterior; los SHAs de aceptación y de squash no se confunden.

| fase | estado | PR/evidencia |
|---|---|---|
| F0 | integrada | #1242 |
| F1 | integrada, adaptación de tests privados | #1243: 488 ficheros a `unit_tests/`; testkit diferido |
| F2 | integrada | #1244: 455 campos agrupados |
| F3-0 | integrada | #1245: 11 grupos de fixture |
| F3-1…F3-12 | integradas | #1246–#1257: 1.726 métodos trasladados |
| F3-I | integrada | #1258–#1260: 1.160 → 443 thunks |
| guía develop-rustycore | integrada | #1253 |
| F4a P1 | integrada | #1261: dispatch_table fuera de SessionCore |
| F4a P2 | integrada | #1262: base test-fixtures y dos regresiones corregidas |
| F4a P3 | integrada | #1264; movimiento `6f0660ab0`, candidato aceptado `002ff5e46` |
| F4a P4a | aceptación completada; publicación/integración en #1263 | candidato `d9c9e3637`, evidencia abajo |
| F4a P4b | pendiente | §6 |
| F4b | pendiente | §6 |
| F5 | diseño aprobado; implementación en curso, sin aceptación | §6 |
| F6 | pendiente | pista de comportamiento |

R5 leído con `python3 -B tools/architecture/wow_world_coupling.py report --json` en P3:
17 miembros superiores, 199.895 líneas clasificadas como producción, 2.598 métodos contabilizados
por el scanner, 443 thunks y 1.726 métodos en subestados. Este reporte textual puede incluir
items con cfg internos; no es un censo del código realmente compilado ni prueba del DAG completo.

El censo manual post F3-I del comentario de #1241 distingue 2.293 funciones de producción y
305 de test dentro de sus 2.598 funciones. Es evidencia histórica con otra clasificación:
378 handlers, 194 API externas, 442/1 thunks producción/test, 50/8 movibles,
88/69 stateless y 1.141/227 bloqueadas. Mantener separados ambos métodos de conteo;
remedir al diseñar F5, sin usar 2.598 como garantía de funciones activas en producción.

### Evidencia de aceptación F4a P3

Host aarch64, Rust 1.98.0, jobs=1, target absoluto del worktree. Base P2 `ec5d6071`;
su árbol coincide con el candidato P2 `66527fab` que produjo el listado histórico.
El movimiento de P3 conserva cuerpos, orden de fases y puntos de llamada; no cambia
paquetes, SQL, persistencia ni owners runtime. Los deltas de baseline se revisaron:
Session crece lógicamente al entrar los helpers en su subárbol; no desaparece evidencia
de persistencia. R1 devolvió **NOT-APPLICABLE**, no PASS de extracción ni ahorro de LOC.

- Final `--base origin/3.4.3 --architecture --timings --logs` en `6f0660ab0`:
  `/tmp/rustycore-p3-final-20261002-acceptance.json`, verde y verificado, 622,487 s.
  Arquitectura/self-test, ownership syntax-only, fixtures físicos, higiene, fmt y
  checks afectados verdes; lib wow-world: 3.901 aprobados / 1 ignorado.
- Extras en ese SHA: cuatro configuraciones `--all-targets` (wow-world default,
  test-fixtures, world-server, world-modules), 47 tests de integración, 1 doc-test,
  597 tests world-server lib y 369 handler-contract-check release aprobados.
  Órdenes, tiempos y salidas en `/tmp/rustycore-p3-20261002.extras.json` y sus logs.
  Listado idéntico de 3.950 identidades (cero altas/bajas), registrado en
  `/tmp/rustycore-p3-20261002-test-identities.json`; hash SHA256 del listado base:
  `cd888877c68a5a2d58cb0dba97a4bcaff74649acd9a457731eb2d5c404df1e22`.
- `compose.py check` detectó un defecto previo del generador: omitía `[lints] workspace=true`
  del manifiesto ya comprometido. El commit separado `002ff5e46` corrige la plantilla,
  con regresión positiva/negativa/restauración. Compose y las 12 fixtures pasan.
  El delta desde `6f0660ab0` solo toca esos dos ficheros Python; las suites Rust extra
  se acreditan en su SHA original, con fuentes/manifiestos/Cargo.lock idénticos.
- Final del candidato corregido, misma orden: `/tmp/rustycore-p3-final-20261002-repaired.json`,
  verde y verificado, árbol limpio, 10:17:19,501–10:19:21,594 UTC, **122,093 s**.
  No se atribuyen los extras anteriores a este SHA. Las posteriores instrucciones/docs
  usan el delta documental `quick --base 002ff5e46`, conservando su propio manifiesto.

Campaña de código: **09:30:05–10:20:07 UTC (3.002 s de pared)**, incluido el primer intento
bloqueado por permisos, extras, listado, reparación y verificación final. Conserva el fallo
de compose previo y el intento de sandbox; no es una campaña de 122 s. El tiempo de
reparación/espera no se midió aisladamente; las dos ejecuciones verdes de final ya suman
744,580 s sin extras. **Objetivo de 600 s incumplido.** La higiene del delta documental
y publicación posteriores tienen su registro separado y no convierten ese coste en verde.
No se desplegó ni reinició runtime, ni se ejecutó QA live o auditoría C++ de comportamiento.

### Evidencia de aceptación F4a P4a

Candidato de código `d9c9e3637f165b438b41c9fb8d2658e6f256f184`, base P3 integrada
`1f8a7c800d56ff3247784148dc94b0dea1c27586`; host aarch64, Rust 1.98.0, jobs=1 y
target absoluto del worktree. La extracción conserva cuerpos, admisión, orden de fases,
paquetes, persistencia y los seis owners de reloj; no retira el MapManager legado.

- Final `--base origin/3.4.3 --architecture --timings --logs --keep-going`: manifiesto
  `target/validation-v2/manifests/20261002T145342.104305Z-2317361-final.json`, verde y
  verificado en ese SHA, árbol limpio, **385,703 s**. Arquitectura/self-test, sintaxis
  ownership, límites físicos, R1, higiene/fmt y workspace `--all-targets` pasan.
  World lib: 3.714 aprobados / 1 ignorado; Core lib: 187 aprobados. El helper ignorado
  sigue siendo `handler_contract_tests::print_world_handler_contract_snapshot`.
- Extras en `/tmp/rustycore-p4a-20261002-d9c9e3637.extras.json`: World default y
  test-fixtures `--all-targets`, 47 tests de integración, un doctest de Core, 597 tests
  world-server lib y 386 tests de handler-contract-check release pasan. Las dos
  configuraciones aisladas de Core se acreditan en `027b7d918`, con fuentes/manifiestos
  y lockfile idénticos; no se relabelan como ejecutadas en el SHA definitivo.
  Compose, sus 12 fixtures, 48 pruebas Python de codemods, dos selftests y los siete
  planes idempotentes pasan. El scanner añade 17 regresiones positivas/negativas de
  ownership, imports y resolución de fachadas; sus 386 tests debug pasan en final.
- La unión World/Core conserva exactamente **3.950 identidades**, cero altas/bajas:
  `/tmp/rustycore-p4a-20261002-d9c9e3637-test-identities.json`. Solo se normalizan la
  ruta del crate y línea del doctest trasladado. La composición del binario world-server
  reporta features vacías para wow-world, wow-world-core y wow-session: sin fixtures ni
  test-support. Ambas configuraciones mantienen cero mensajes de warning nuevos tras
  normalizar los imports trasladados; encabezados diagnósticos default 415 → 405,
  fixtures 464 → 451. Son emisiones de Cargo, no un censo de deuda única.
- R1: **S=14.372, G=15.385, allowance=15.390,6**, con ratio 5% + slack 300 intactos.
  Session root queda en su techo de 1.052 líneas; protocolo Core tiene 967 bajo el techo
  de 1.000. No se amplían excepciones físicas. La arista Core → wow-handler transporta
  únicamente PacketUpdatePhase y no crea otro scheduler. Core no depende de World/Session.

El inventario exhaustivo pasa con 9.939 registros: 7.718 de producción y 2.221 de fixtures,
cuatro entradas generadas y 1.034 grupos (1.031 workflows de producción + tres de fixtures).
Su ejecución se registra en los extras, con sus tiempos y salida reales. El primer intento
había fallado por drift heredado: **190 identidades añadidas y 88 obsoletas**, con 9.749 filas
exactamente conservadas. Los seis archivos afectados no cambian entre la base y P4a:
app, administración de personajes, catálogos de skills/Trait, name query y sus dos módulos
de statements. Se revisaron las huellas y orden actuales antes de actualizar snapshot,
anotaciones y política; no se borraron filas por dejar de inspeccionar el destino.

La revisión de procedencia usa Trinity 3.4.3 en
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`: HotfixDatabaseConnection::DoPrepareStatements
(`src/server/database/Database/Implementation/HotfixDatabase.cpp`, Skill/Trait), load info
Skill/Trait (`src/server/game/DataStores/DB2LoadInfo.h`) y DB2Manager::LoadHotfixData
(`src/server/game/DataStores/DB2Stores.cpp`); CharacterCache::LoadCharacterCacheStorage
(`src/server/game/Cache/CharacterCache.cpp`) y PlayerGuidLookupData::Initialize
(`src/server/game/Server/Packets/QueryPackets.cpp`). Son anclas de los flujos previamente
integrados de #524/#807, no nueva prueba de paridad. TraitDefinitionEffectPoints y la
aceptación restante de #524 conservan sus límites en STATE/refactor-completion-plan.
La metadata conserva consultas de startup Character → Login antes de publicar el cache,
actualizaciones de cache tras las ramas SQL de éxito y los errores existentes, incluido
el ownership-query error que el borrado actual ignora. No se repara gameplay ni se infiere
atomicidad o resolución de unknown-COMMIT.

R5 en ese SHA: **185.925** líneas clasificadas como producción
(frente a 199.895 en P3), 2.598 métodos, 443 thunks y 1.726 métodos en subestados.
El reporte sigue siendo textual: sus cfg internos no prueban un censo compilado ni el DAG completo.

Campaña de código **11:55:50–15:19:37 UTC, 12.227,640 s de pared**;
registro completo: `/tmp/rustycore-p4a-20261002-campaign.json`. Conserva cuatro finales
fallidos (97,236 / 563,987 / 1.135,966 / 536,889 s), el final verde intermedio en
`027b7d918` (545,879 s) que todavía no aceptaba la fase por el gate exhaustivo fallido,
y el final interrumpido en `d4f5dab8d` (336,861 s, exit 130) para precisar la metadata
revisada del borrado. Ventanas de reparación/revisión registradas aparte; algunas marcas
son aproximadas y no se midió CPU de reparación de forma aislada. Los costes de inventario
son explícitos: primer check fallido 779,910 s, ventana observable del diagnóstico
890,159 s (no tiempo aislado de CPU), check aceptado 784,958 s.
El candidato definitivo suma **691,218 s de órdenes ordinarias**, aun excluyendo ese
último inventario. **Objetivo completo de 600 s incumplido.** No se ocultan los intentos
previos, reparaciones, diagnósticos ni extras detrás de los 385,703 s del final verde.
La posterior documentación/publicación tiene su registro separado y no relabela este SHA.

La publicación/integración de la fase se registra en #1263 bajo la autoridad conservada.
P4b, F4b, F5 y F6 siguen pendientes; esta aceptación no cierra #1263/#584 ni demuestra
ahorro de build. No se desplegó ni reinició runtime ni se reclama QA live.

### Estado F5 en curso — 2026-10-04, checkpoint `5c6afbb50` (NO VALIDADO)

La rama de continuación `1263-f4a-p4b-hub` partía de un WIP que no compilaba en
`wow-world-core`. Tras reparar conexiones de fuente (montajes/exports, rutas de datos,
constantes compartidas, atributos `cfg` colgantes, llaves huérfanas y préstamos), el estado
verificado con el bucle de feedback del implementador (`CARGO_BUILD_JOBS=1`, target del
worktree, `PROTOC` fijado, sin campaña de aceptación) es:

- `cargo check --workspace --all-targets`: **verde**.
- `cargo test -p wow-world --lib`: **3.685 passed / 0 failed / 1 ignored**.
- `cargo test -p wow-world-application`: 29/29.

Sobre el checkpoint `5c6afbb50` (fanout de botín duradero en App) se repitió el mismo
conjunto: `cargo check --workspace --all-targets`, `wow-world --all-targets` (con y sin
`test-fixtures`) y `wow-world-application --all-targets` verdes; `cargo test -p wow-world
--lib` **3.687 passed / 0 failed / 1 ignored**; `wow-world-application` 29/29; sin avisos
nuevos de imports ni de código muerto.

Sobre el checkpoint `c092a4811` (destroy de item looteado en App) se repitió el mismo
conjunto: `cargo check --workspace --all-targets`, `wow-world --all-targets` (con y sin
`test-fixtures`) y `wow-world-application --all-targets` verdes; `cargo test -p wow-world
--lib` **3.687 passed / 0 failed / 1 ignored**; `wow-world-application` 29/29; sin avisos
nuevos de imports ni de código muerto. La rama ya trae F4a P4b (hub en `wow-world-core`) y
los crates de dominio F4b; lo que queda de F4–F6 es F5 (handlers/orquestación) y F6.

Sobre el checkpoint `63b30b09d` (refresco de cofre en App) se repitió el mismo conjunto:
`cargo check --workspace --all-targets`, `wow-world --all-targets` (con y sin
`test-fixtures`) y `wow-world-application --all-targets` verdes; `cargo test -p wow-world
--lib` **3.687 passed / 0 failed / 1 ignored**; `wow-world-application` 29/29; sin avisos
nuevos de imports ni de código muerto. Además se ejecutó el bloque de inventario ya escrito
(`cargo test -p wow-world --lib -- scenarios_player_items raw_equip`: 179/179), que hasta
ahora figuraba como "escrito, no ejecutado" en este documento.

Sobre el checkpoint `b044cbcee` (decisión de CompleteQuest en App + paridad C++) la
verificación del implementador fue: `cargo check --workspace --all-targets` verde,
`cargo check -p wow-world --all-targets` y `--all-targets --features test-fixtures` verdes,
`cargo check -p wow-world-application --all-targets` verde, `cargo test -p wow-world --lib`
**3.687 passed / 0 failed / 1 ignored** (3.685 del baseline + 2 escenarios nuevos de
paridad), `handlers::quest` 228/228, `scenarios_quest_2` 8/8 y
`cargo test -p wow-world-application` 29/29. Sin avisos nuevos de imports sin usar ni de
código muerto respecto del baseline (comparación de conjuntos de nombres). No se repitieron
otras suites ni la campaña `final`.

RawEquip (F5) está conectado por fuente y validado de forma acotada:
`RegistrySyncInputs` inerte en Core presta los vitals de Stats al final; `InventoryEquipCxLikeCpp`
montado y exportado; sus tres escenarios (`raw_equip`) pasan. Se corrigieron además tres
defectos de producción hallados al ejecutar las pruebas nuevas del WIP: el slot de
recalculación de estadísticas de combate usaba constantes sin cualificar
(`wow-world-inventory/src/storage.rs`), y `apply_offline`/`take_rested_xp` resolvían el
fixture en vez del Player canónico por GUID (`session/catalogs/operations.rs`,
`session/rest_progression.rs`).

Barreras explícitas que siguen abiertas, sin declarar cierre:

- **LootRelease**: `crates/wow-world-application/src/loot_release/` está montado
  (`mod loot_release;`) y **tiene consumidor World**. `WorldSession::loot_release_cx_like_cpp`
  construye el Cx con préstamos disjuntos; `do_loot_release_all_like_cpp` y
  `do_loot_release_owner_like_cpp` (entrada de `requests.rs`, `persistence.rs`, cleanup,
  finalization, swaps y spell ops) delegan en `release_all_like_cpp`/`release_owner_like_cpp`
  de App. Para llegar ahí: la geometría compartida
  `represented_gameobject_interaction_distance_like_cpp` y
  `represented_gameobject_display_box_contains_like_cpp` vive en `wow-world-entities` (con
  reexport en `handlers::loot` para consumidores y pruebas existentes); el Cx recibe
  `quest_state`, `social`, `spell_state` (cfg) y `SessionFixtures` **mutable** (cfg) y
  construye `HubRef` inerte por `owner.core_ref_like_cpp()` reutilizando `catalogs`/`config`
  de `LootReleaseStatsInputsLikeCpp`; se implementaron
  `represented_gameobject_can_autostore_loot_item_like_cpp` (posición canónica/representada,
  display box, rango de spell-lock con known-spells y fallback de fixture) y
  `send_gathering_node_loot_release_dynamic_flags_update_like_cpp` (proyección
  `ViewerDependentValue<ObjectData::DynamicFlagsTag>` con `QuestEligibilityCx` y condición
  construidas bajo demanda).
- **Resolución del solapamiento de fixtures** (el Cx no era construible con el diseño previo):
  `LootReleaseStatsInputsLikeCpp` ya no guarda `StatsFixtureRefs` con `&mut`; conserva solo
  catálogos/config y `stats_like_cpp` recibe race/class/level y el `StatsFixtureRefs`
  reborrow en la llamada. Así `SessionFixtures` queda como **un único dueño mutable** en el
  Cx, que sirve a la vez la pasada de stats (`send_stat_update`), la proyección de condición
  y `sync_player_registry_state_like_cpp` (que ahora lee `self.fixtures` directamente y ya no
  necesita `LootReleaseRegistryFixtureRefsLikeCpp`, tipo retirado). Se eliminaron también las
  copias World ya sin uso `canonical_creature_fully_looted_after_represented_sync_like_cpp`,
  `canonical_gameobject_fully_looted_after_represented_sync_like_cpp` (WorldSession) y
  `LootCx::canonical_gameobject_is_fully_looted_like_cpp`.
- **Pendiente de LootRelease**: ya **no** queda copia World del release ni del cierre
  diferido. `finalize_unviewed_durable_loot_owner_like_cpp` (`handlers/loot/persistence.rs`)
  delega en el nuevo `LootReleaseCxLikeCpp::release_detached_owner_like_cpp`, que reinserta
  el snapshot comprometido en la caché y corre las mismas transiciones bajo la observación
  *unviewed* (`set_canonical_gameobject_loot_state_if_unviewed_...`, la nueva
  `LootReleaseOwnerAccessLikeCpp::finish_unviewed_looted_creature_like_cpp` —que comparte el
  cierre de ciclo de vida con la variante *viewed*— y la compuerta de corpse ya existente en
  Core). Se retiraron las copias World `apply_represented_gameobject_loot_release_like_cpp`,
  `hide_...`, `send_gathering_node_...`, `send_creature_loot_release_dynamic_flags_update_...`,
  `remove_canonical_corpse_lootable_dynamic_flag_if_unviewed_...` y el struct
  `AuthoritativeLootReleaseLikeCpp`, con imports de test movidos a `#[cfg(test)]`.
  Tampoco queda duplicado de la compuerta de autostore: se extrajo como provider libre
  `wow_world_application::represented_gameobject_can_autostore_loot_item_like_cpp` (posición
  canónica/representada, display box y rango de spell-lock con `known_spells` explícitos) y
  `handlers/loot/handlers/item.rs` la consume por la fachada World; se retiraron las copias
  World del gate y de `represented_gameobject_spell_lock_range_like_cpp`. También se
  convergió el comando de refresco de estado de cofre: provider libre
  `queue_chest_gameobject_state_refresh_for_same_map_like_cpp` en App, consumido por la
  fachada `LootCxRef` de World (la usan `session/world_entities/gameobject.rs` y
  `handlers/loot/sources/gameobject.rs`), retirando el constructor World
  `chest_gameobject_state_refresh_command_like_cpp`. Se convergió además el destroy de item
  directo totalmente looteado: `item_storage.rs::destroy_fully_looted_direct_item` construye
  el Cx App y delega; se retiró la copia World de 108 líneas
  `destroy_direct_item_count_after_loot_release_like_cpp` y su helper libre
  `direct_item_count_after_loot_release_like_cpp` (ahora `pub` en App para la prueba de la
  aritmética). Y la publicación del fanout de botín duradero más el cierre del dueño
  *unviewed* pasaron al Cx App (`loot_release/fanout.rs`:
  `publish_durable_loot_item_fanout_like_cpp`, `finalize_unviewed_durable_loot_owner_like_cpp`
  y el helper libre `durable_loot_item_fanout_viewers_like_cpp`); World solo prepara la ruta
  y llama. El módulo App todavía no tiene pruebas propias (la cobertura la aportan hoy las
  264 pruebas `handlers::loot` de `wow-world`, que ahora ejercitan el camino App).

**Límite de este método**: retirar duplicados ya convergidos no alcanza el objetivo de F5
(20–40k líneas en `wow-world`, hoy ~128k). F5 exige el rediseño de registro descrito en
#1263 §5 (handlers como funciones de crate de dominio sobre `<Domain>Cx` registradas por
`register()`); en esta rama `PacketHandlerEntry` sigue siendo
`<WorldSession, SessionHandlerCatalogsLikeCpp>` y ningún crate de dominio expone `register()`.
Ese rediseño no está iniciado.
- El `QuestGameObjectVisibilityCx` readonly App (`quest/visibility/gameobject_flags.rs`,
  completions en `quest/objectives.rs`) tiene consumidor World completo: ActivateToQuest,
  DynamicFlags y el refresh `update_visible_gameobjects_like_cpp` (que además usa
  `represented_has_quest_for_gameobject_like_cpp`/`represented_gameobject_is_for_quests_like_cpp`)
  delegan en los providers App, ahora `pub`. Las copias World de esas dos consultas quedaron
  como fachadas finas sobre el mismo provider, sin lógica duplicada.
- **CompleteQuest** (primer corte): la decisión de diálogo de
  `handle_quest_giver_complete_quest` vive ahora en App
  (`quest/complete.rs`: `represented_quest_complete_dialog_like_cpp`,
  `represented_quest_has_item_objective_like_cpp`, `represented_quest_rewards_block_like_cpp`),
  y World solo evalúa los seams acotados `can_reward_quest`/`can_complete_repeatable_quest` y
  publica. Al contrastar con `HandleQuestgiverCompleteQuest`
  (`/home/server/woltk-trinity-legacy/src/server/game/Handlers/QuestHandler.cpp:533-585`) se
  corrigió una divergencia real: una quest COMPLETE **con objetivos de objeto** responde
  `SendQuestGiverRequestItems` (no `SendQuestGiverOfferReward`), y el bloque de recompensas y
  los `status_flags`/`collect` provienen ya de los helpers compartidos en vez de construirse
  a mano con `collect` vacío y `0xFD` fijo. Quedan sin mover a App los seams acotados de
  `CanRewardQuest`/`CanCompleteRepeatableQuest` y la publicación de paquetes de diálogo, y
  persiste una diferencia de guarda no corregida: C++ solo rechaza cuando
  `!CanSeeStartQuest(quest) && GetQuestStatus == QUEST_STATUS_NONE`, mientras el handler usa
  `has_quest(quest_id)` (exige una entrada de estado), de modo que una quest visible pero no
  aceptada no recibe el diálogo. Se deja explícita, sin cambiar el comportamiento en este
  corte.
- StorageMove/swap/child/offhand como executor completo, Save y compra Trainer conservan
  pendientes.
- Sin `final`, arquitectura/self-test, inventario de persistencia, capturas ni live; por
  tanto no hay aceptación de F5/F6 ni cierre de #1263.

## 9. Herramientas

- `tools/architecture/wow_world_coupling.py`: mapa de acoplamiento (campos por dominio, campos
  hub, aristas de métodos entre dominios, violaciones del DAG) y métrica R5.
- `tools/architecture/net_move.py`: comprobación R1 net-move
  (`python3 tools/architecture/net_move.py check --base origin/3.4.3`).

## 10. Aceptación por PR, sin repetir evidencia

Esta sección concreta R3 para #1263 y reemplaza la receta de repetir pasos 1–8 del antiguo
checklist de develop-rustycore, alineado en esta entrega. La guía remite a este documento
y no crea otro criterio de aceptación.

1. Planificar una campaña sobre el candidato comprometido y registrar SHA, base, estado del
   árbol, configuraciones y blancos. Reservar un ejecutor; comprobar procesos, RAM y disco.
   Mantener `PROTOC`, `CARGO_BUILD_JOBS=1`, `VALIDATION_V2_CARGO_JOBS=1` y el target absoluto
   del worktree; no compartir caché con worktrees activos.
2. Ejecutar una vez
   `./tools/validation-v2 final --base origin/3.4.3 --architecture --timings --logs`.
   Verificar el manifiesto exacto de esa invocación con `--require-profile final` y cotejar
   `provenance.head`, raíz, base y estado con el candidato. Ausencia de manifiesto o
   cualquier fallo es aceptación fallida. Respetar las reglas documentadas para deltas
   exclusivamente documentales; no relabelar un SHA anterior.
3. Leer las órdenes y suites realmente ejecutadas en el manifiesto. Acreditar las suites
   completas de librería, checks afectados, arquitectura/self-test, sintaxis ownership,
   fmt, whitespace, fuentes ignoradas y R1 que ya cubra. Añadir solo evidencia no cubierta:
   los blancos de integración/doc-tests exigidos y las configuraciones que falten de
   `wow-world --all-targets`, `wow-world --all-targets --features test-fixtures`,
   `world-server --all-targets`, `world-modules --all-targets` y los crates trasladados.
   Mantener cero warnings nuevos respecto de la base en la misma configuración.
4. Preservar la suite completa de `wow-world`, la de cada dominio trasladado,
   `cargo test -p world-server --lib` y los tests release de handler-contract-check.
   No repetir una suite/configuración idéntica ya acreditada por final. Conservar
   pruebas de módulos/hooks y el fixture `tools/modules/compose.py check` con sus argumentos
   vigentes; final no los sustituye automáticamente.
5. Antes/después: comparar identidades de test, no solo totales. P3 conserva la lista de
   `wow-world`; desde P4a comparar la unión de crates origen/destino, con un mapa de
   cambios de crate/módulo/nombre, cfg/features y estado ignored. Los 3.950 tests son la
   base histórica de F1/P3, no una cifra perpetua por crate. Registrar cada adición o
   retirada intencional; no aceptar pérdida de escenarios por mover cfg o fixtures.
   Separar tests bajo fixtures de pruebas con composición de producción.
6. Añadir el inventario exhaustivo y los gates de referencias/snapshots cuando una fase
   mueva accesos de persistencia; incluir aceptación de producción, capturas y runtime
   según el cambio. En el cierre físico de las responsabilidades afectadas aplicar los
   presupuestos y excepciones de module-design-guidelines, con evidencia terminal
   acotada al alcance; no declarar terminado #584 por un PASS de migración.
7. Medir desde el primer check requerido hasta el último, incluidos extras y listados.
   Registrar inicio, fin y duración total, además de la duración de final. Más de 600 s
   incumple el objetivo de rendimiento aunque la corrección sea verde. Bootstrap frío,
   auditoría exhaustiva y QA live se etiquetan como costes distintos; cualquier coste
   adicional se informa, sin ocultarlo en campañas nominalmente separadas.
   Tras fallos, reparar el conjunto relacionado y repetir solo la evidencia afectada.

## 11. Helper local de orquestación: pendiente de reparación

`/home/server/rustycore-1241-orchestration/phase.sh` no es actualmente una barrera fiable:
solo usa `set -u`, imprime varios códigos y continúa, usa tuberías sin preservar los fallos,
elige el manifiesto más reciente y su operación merge no exige evidencia del candidato.
Esta revisión cambia el plan; no ha reparado ni ejecutado el helper.

Antes de usarlo para aceptar o integrar:

- Propagar los códigos de todas las órdenes; una tubería o un mensaje LIST-DIFFERS no puede
  transformar un fallo en salida 0. Si se acumulan diagnósticos, el resultado final sigue rojo.
- Capturar el manifiesto de la invocación actual y vincularlo al SHA/base/worktree exactos,
  junto con resultados de extras y el conjunto de tests. Nunca recuperar evidencia antigua
  cuando falte el manifiesto actual.
- Bloquear push/merge automatizados sin evidencia vigente, verde y correspondiente a la rama;
  conservar la autoridad separada de publicación/runtime, no concederla desde el helper.
- Registrar inicio/fin y duración de la campaña completa y quitar repeticiones según §10.
- Integrar el contrato y la herramienta útil en el repositorio, con casos negativos de fallo
  de final, verify, tests, listado y SHA incorrecto; no depender solo de una ruta privada del host.

P3 se acepta con las órdenes canónicas y el registro explícito de §8. Las fases siguientes
mantienen este procedimiento hasta reparar el helper.

## Nota histórica

La versión anterior de este documento era el plan de #1233 ("forma modular estilo AzerothCore",
F0-F13, rama `584-wow-world-distribution`); queda en el historial de Git y no es un plan activo.
