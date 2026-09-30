# Distribución de `wow-world` — programa #1241

**Estado:** F0 en curso. **Autoridad:** #1241, hija de #584; el plan técnico general sigue siendo
[refactor-completion-plan.md](refactor-completion-plan.md). Este documento es el registro de
progreso del programa y **sustituye** el enfoque anterior de #1233 (una rama, validación
diferida); no hay un segundo plan activo. **No reclama** paridad, ahorro de build ni cierre de
issue.

Base: `3.4.3` @ `e786ece1`. Una rama y un PR por fase (F0, F1, ...) o por dominio, integrados de
forma continua en `3.4.3`. La excepción correspondiente de proceso está en
[AGENTS.md](../../AGENTS.md) ("#1241 wow-world split programme").

Referencia estructural vigente (capas, nombres, visibilidad, colocación de tests, presupuestos):
[structure-and-conventions.md](structure-and-conventions.md). El orden global del workspace vive
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
3. **Registro invertido:** cada crate expone `pub fn register(registry: &mut HandlerRegistry)` y
   `world-server` los compone. `PacketHandlerEntry` sigue siendo la única fuente de registro y
   llamada.
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

## 6. Fases y aceptación

| fase | alcance | aceptación |
|---|---|---|
| F0 | Base y herramientas: aparcar la rama, mapa de acoplamiento, net-move (R1) planificado por `tools/validation-v2` en quick/final cuando cambia `crates/wow-world/src/`, enmienda de `AGENTS.md`/skill y este documento | herramientas con tests; documentos sin plan competidor |
| F1 | Tests fuera de `src`: los que usan API pública o `test-fixtures` a `crates/wow-world/tests/`; fixtures compartidos a `wow-world-testkit`; los de acceso privado quedan `#[cfg(test)]` en ficheros hermanos | `wow-world/src` ≤ ~190k líneas; mismo número de tests (antes/después registrado); suites movidas verdes |
| F2 | Sub-estados: agrupar los 455 campos en ~25 `<Dominio>State` + `SessionCore`, por codemod, aún dentro de `wow-world` | `WorldSession` = `core` + ~25 campos; compila; suite completa de `wow-world` verde |
| F3 | Métodos a sub-estados, de abajo arriba en orden DAG: `impl WorldSession` -> `impl <Dominio>State` con `<Dominio>Cx`, dejando thunks de una línea; un PR por dominio | métodos `impl WorldSession` bajan de 3 834 hacia ~uno por handler/thunk |
| F4 | Extracción: cuando un dominio ya no nombra `WorldSession`, `git mv` a `crates/wow-world-<dominio>/` con `register()`; movimiento puro (R1) | `wow-world` queda como cáscara (core, driver, composición) |
| F5 | Registro dentro de los dominios: thunks a sus crates mediante traits extractores (`HasLoot`, `HasItems`, ...) definidos en un crate API bajo e implementados por `WorldSession` | `wow-world` con 20k-40k líneas de producción |
| F6 | Pista de comportamiento, separada: por dominio, elegir y borrar `represented_*` o canónico; decidir `session/legacy_runtime` (8 432 líneas) | evidencia de paridad por retirada (anclas C++, capturas y QA live donde se requiera) |

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

## 8. Estado

| fase | estado | PR | métrica R5 al cerrar |
|---|---|---|---|
| F0 | en curso | — | — |
| F1 | pendiente | — | — |
| F2 | pendiente | — | — |
| F3 | pendiente | — | — |
| F4 | pendiente | — | — |
| F5 | pendiente | — | — |
| F6 | pendiente | — | — |

## 9. Herramientas

- `tools/architecture/wow_world_coupling.py`: mapa de acoplamiento (campos por dominio, campos
  hub, aristas de métodos entre dominios, violaciones del DAG) y métrica R5.
- `tools/architecture/net_move.py`: comprobación R1 net-move
  (`python3 tools/architecture/net_move.py check --base origin/3.4.3`).

## Nota histórica

La versión anterior de este documento era el plan de #1233 ("forma modular estilo AzerothCore",
F0-F13, rama `584-wow-world-distribution`); queda en el historial de Git y no es un plan activo.
