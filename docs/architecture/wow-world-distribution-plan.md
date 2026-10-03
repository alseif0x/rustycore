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

### F6 — retirada de duplicados, pista de comportamiento

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
