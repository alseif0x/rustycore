# Distribución de `wow-world` — programa #1241, continuación #1263

**Aceptación P4a — 2026-10-02:** F0–F3 y F4a P1–P3 integradas;
P4a aceptada en `d9c9e3637`, con publicación/integración registradas en #1263.
**Responsabilidad pendiente:** [#1263](https://github.com/alseif0x/rustycore/issues/1263),
continuación de [#1241](https://github.com/alseif0x/rustycore/issues/1241) bajo #584.
#1241 ya está cerrada en GitHub; ese estado no demuestra que F4–F6 estén terminadas.
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
los targets unitarios/integración y el grafo de producción sin fixtures. Está todavía
sin validar. La solución de P4a mediante dev-dependency de Core no probaba este nuevo
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
lista sin alterar el flag. No hay todavía implementación ni aceptación F4b.
El contraste de proveedores en `99d86c533` confirma que `HubRef`/`HubMut` y
`SessionCommand`/payloads de duelo pertenecen ya a Core; no bloquean el traslado
de los nueve impls sociales. El cierre World real incluye las DTO de
calendar/petitions/duel, el evento de force-deselect, el enum de reconciliación,
la distancia de XP de grupo y las constantes de duelo/guild que comparten los
wrappers. Conservar sus fachadas necesarias, el cuerpo de flood con su `HubMut`
Core y el puente `test-fixtures` aprobado; los shells/builders siguen en World.

### F5 — handlers y orquestación

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

### F6 — retirada de duplicados, pista de comportamiento

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
| F5 | diseño detallado e implementación pendientes | §6 |
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
