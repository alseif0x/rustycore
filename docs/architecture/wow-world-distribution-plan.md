# Plan de distribución de `wow-world` — forma modular estilo AzerothCore

**Estado:** programa en ejecución, nivel 1 (implementación sin campañas intermedias; una única
validación final). **Autoridad:** macro #1233 bajo #584; el plan técnico general sigue siendo
[refactor-completion-plan.md](refactor-completion-plan.md) y aquí solo se detalla la forma, los
presupuestos y la secuencia de esta rebanada. **No reclama** paridad, ahorro de build ni cierre
de issue.

Base medida: `3.4.3` @ `9daa13f6`. Trabajo en la rama `584-wow-world-distribution`.

El estándar estructural que gobierna estos cortes —capas, nombres, visibilidad, colocación de
tests, presupuestos y checklist de cambios— es
[structure-and-conventions.md](structure-and-conventions.md). Regla central de ese estándar: la
paridad con la referencia C++ es **de lógica**, no de estructura; ningún corte se justifica con
"en C++ es así".

El orden global —incluidas las olas de saneamiento del resto del workspace y las dependencias
entre fases— vive en [workspace-structure-programme.md](workspace-structure-programme.md); este
documento detalla solo `wow-world`.

## 1. Problema, con números

| medida | valor |
|---|---:|
| líneas del workspace (41 crates) | 881 133 |
| `wow-world` | **422 134 (48 %)**, 767 ficheros |
| ├ `session/` | 218 146 (369 ficheros; `tests/` 118 498) |
| ├ `handlers/` | 143 016 (237 ficheros) |
| └ `handlers/misc/` (vertedero sin dueño) | 21 154 (48 ficheros, 20 módulos) |
| owner lógico mayor (`session`, según el ratchet) | **223 952 líneas** |
| bloques `impl WorldSession` | **221**, y **915** referencias al tipo |
| crates vacíos/stub | 6 (`wow-combat`, `wow-spell`, `wow-achievement`, `wow-pvp`, `wow-scripts`, `world-modules`) |
| inversiones de dependencia | 5 (`packet→loot/movement`, `data→entities/movement`, `entities→loot`, `map→loot`, `world-modules→world-server`) |

Evidencia interna de que el tamaño no es una opinión: en la medición de #1236 (checkout aislado,
documentada en ese PR) el fichero `creature_melee_tick.rs` de 1 608 líneas hizo que un worker
tardara 19 minutos sin editar nada; partido en cuatro módulos, la primera edición llegó a los
6 minutos.

Precedente externo: proyectos grandes de Rust trocean **por capas y frecuencia de cambio**, dejan
la capa de aplicación delgada y usan crates como cerca del compilador (`rustc`, `rust-analyzer`,
`bevy`, `zed`); un crate de ~250 k líneas se considera problema y se descompone
([agaric #2878](https://github.com/jfolcini/agaric/issues/2878)), y la forma del workspace afecta
la latencia de herramientas ([slint #12097](https://github.com/slint-ui/slint/issues/12097)). La
guía oficial de Rust reserva **módulos** para organizar dentro y **crates** para unidades
independientes con API propia (The Rust Book, cap. 7).

## 2. Forma objetivo

### 2.1 Grafo de crates (las flechas solo bajan)

```
CAPA 5  composición      world-server · bnet-server
CAPA 4  aplicación       wow-world            (Session, adaptadores, ticks, publicación)
CAPA 3  reglas           wow-combat · wow-spell · wow-spell-acquisition · wow-conditions
        deterministas    wow-loot · wow-quest · wow-items · wow-pets · wow-social
        sin Session      wow-economy · wow-instances · wow-battlegrounds
        sin I/O          wow-dungeon-finding · wow-progression · wow-ai · wow-account-collections
CAPA 2  modelo/estado    wow-entities · wow-map · wow-movement · wow-packet · wow-network
CAPA 1  datos/durables   wow-data (solo catálogos) · wow-persistence · wow-database
CAPA 0  base             wow-core · wow-constants · wow-config · wow-crypto · wow-logging
                         · wow-math · wow-proto
```

Objetivo: **12-14 crates de dominio con contrato real**; el resto de familias de handler son
**directorios** dentro de la app (como `game/` en AzerothCore o los módulos internos de
`tokio`/`datafusion`). Ni un crate por carpeta ni un monolito.

### 2.2 Skeleton de la aplicación

```
wow-world/src/
├─ session/
│  ├─ state/         WorldSession + SUB-ESTADOS DUEÑOS (SessionTransport, SessionPlayerAccess,
│  │                 SessionLoot, SessionSpells, SessionPersistenceFence, SessionCatalogs)
│  ├─ construction.rs · admission · lifecycle/ · driver/ · persistence/ · publication/
│  ├─ directory.rs · registry.rs (fuente única de registro de opcodes) · policy.rs
│  ├─ handlers/      ADAPTADORES por familia de opcode: decode → dominio → aplicar → publicar
│  └─ tests/         escenarios de integración + fixtures compartidos (test_support)
└─ runtime/          orquestación de ticks (el estado de mapa pertenece a wow-map)
```

### 2.3 Skeleton de un crate de dominio

```
wow-<dominio>/src/
├─ model.rs     datos del dominio (no conoce Session)
├─ rules/       funciones puras: entradas → decisión o intención
├─ adapter.rs   puente de importación del consumidor (temporal; se retira al migrar)
└─ tests/       escenarios focales junto a la regla que prueban
```

### 2.4 Presupuestos duros (se hacen cumplir, no se revisan)

| unidad | límite | hoy |
|---|---:|---:|
| fichero físico | 1 000 líneas (objetivo 400-600) | máx. 1 716 |
| owner lógico (ratchet) | 20 000 líneas | `session` 223 952 |
| crate de dominio | ~60 000 líneas | `wow-world` 422 134 |
| capa de aplicación | ~200 000 líneas | 422 134 |
| módulo sin dueño (`misc`/`utils`/`common`) | 0 | 21 154 |

## 3. Reglas de corte

1. **Clasificar cada hoja del grafo**: *regla pura* (no menciona `WorldSession`, no hace I/O) → se
   va al crate de dominio con sus tests; *estado* → se queda tras contrato estrecho; *orquestación*
   (vida, driver, persistencia, publicación) → se queda en la app.
2. **Nunca cruza un `&mut WorldSession`**: el dominio recibe un DTO y devuelve una **intención**;
   la app la aplica. Un solo escritor, un solo lock, una sola autoridad por transición.
3. **Crate solo si**: contrato independiente + dirección forzada por el compilador + (≥2
   consumidores o aislamiento de compilación útil). Si no, módulo privado.
4. **Movimiento y comportamiento separados**: ninguna reparación de gameplay dentro de una fase de
   movimiento.
5. **Prohibido** crear espejos de estado, contextos universales, traits por helper, campos públicos
   o locks nuevos para poder mover código.

## 4. Fases

Cada fase es un commit coherente en la misma rama, con el compilador como feedback
(`cargo check -p <crate>`, `CARGO_BUILD_JOBS=1`) y sin campañas intermedias.

| fase | alcance | salida verificable | tipo |
|---|---|---|---|
| F0 | Resync del ledger de hotspots (estaba rojo en `3.4.3`) | `hotspot-ratchet` PASS | mecánico |
| F1 | Reparar la base de #1233 (no parseaba; 898 errores) y portar el split de melee | lib + tests compilan, 0 errores | mecánico |
| F2 | Retirar la autoridad duplicada de personal-phase (canónica en `wow-map`) | 547 líneas fuera, builds verdes | mecánico |
| F3 | `phasing` → `wow-map` | ambos crates verdes; tests viajan con el código | mecánico |
| F4 | Fixtures de test → `handlers/test_support/` | 19 escenarios preservados, cuerpos idénticos | mecánico |
| F5 | `handlers/misc` → 13 directorios de dominio | sin `misc`; 0 errores; sin cambio de comportamiento | mecánico |
| F6 | Dominios ya cohesivos → su crate: `reputation`→`wow-progression`, `spell_acquisition::planner`, `profession`/`trainer`, `entity_update_bridge`→`wow-entities`, `battle_pet_*`→`wow-pets` | crate + tests por dominio | mecánico |
| F7 | **Romper el tipo Dios**: sub-estados dueños dentro de `WorldSession` (transport, player-access, loot, spells, persistencia) | ninguna familia de handler comparte ya un `&mut self` de 100 k | **diseño** |
| F8 | Handlers grandes → adaptadores (`items` 3 904, `loot/sources` 3 110, `world_entry` 2 782, `quest/handlers` 2 714, `session_state` 2 705, `loot/requests` 2 544) | cada adaptador ≤600 líneas; regla y test en su dominio | **diseño** |
| F9 | `map_manager` + `map_manager_tests` → `wow-map` **con contrato** (hoy 93 y 134 referencias desde sesión) | ciclo ausente; tick y publicación con un dueño explícito | **diseño** |
| F10 | Tests: escenarios de dominio a su crate; integración en la app | cada crate prueba lo suyo; `test_support` sin deps de `misc` | mecánico |
| F11 | Grafo: romper `packet→loot/movement`, `data→entities/movement`, `entities→loot`, `map→loot`; llenar o retirar los 6 stubs; invertir `world-modules→world-server` | grafo acíclico por capas | diseño menor |
| F12 | `world-server/app.rs` (5 645 líneas, 151 `await`) → módulos por fase | composition root sin funciones gigantes | diseño |
| F13 | Deuda: 180 warnings; regenerar la política de ownership (revisada, no a ciegas); registrar/cerrar los 2 huecos de comportamiento de phasing | política consistente con el árbol | mecánico |

## 5. Medición y criterio de éxito

- **Navegabilidad (segura)**: fichero máximo ≤1 000; `wow-world` ≤200 000; sin `misc`; owner lógico
  máximo ≤20 000; ninguna familia de handler sobre un `&mut self` de 100 k.
- **Build (a medir, no a prometer)**: protocolo de #1231 (check/build frescos y tibios,
  `--timings`, mediana y rango, mismo host y toolchain). Con `CARGO_BUILD_JOBS=1` la ganancia de
  reloj puede ser nula o negativa: **un resultado negativo se publica como negativo** y no se usa
  para justificar más troceo.
- **Herramientas**: latencia de `rust-analyzer` antes/después (motivo del caso slint).
- **Corrección**: ninguna fase cambia bytes de paquete, orden de fases, admisión ni persistencia;
  cualquier reparación de comportamiento va en su propio commit con su ancla C++ y su prueba.

## 6. Cumplimiento y proceso

- Una sola rama y un commit por fase; el ledger y la política físicas se actualizan con **delta
  revisado** (nunca regenerando a ciegas ni subiendo un techo sin retirada validada).
- Los dos huecos de comportamiento detectados al auditar la duplicación de personal-phase quedan
  **fuera** de los movimientos y documentados: el `PhaseShift`/`PhaseRef` reducido de `wow-map`
  frente al de `wow-entities` (cambia `has_personal_phase`), y el registro por objeto +
  `InitDbPersonalOwnership` que C++ hace en `ObjectGridLoader::LoadHelper` y el `LoadGrid` de
  `wow-map` no (`ObjectGridLoader.cpp:119-126`), más la decisión sobre `map->Balance()`.
- Nada se publica (push/PR/merge) sin autorización explícita; la aceptación es **una única campaña
  final** cuando el árbol esté distribuido, con `./tools/validation-v2 final --architecture`.

## 7. Estado a la fecha de este documento

Hechas F0-F4 (commits `c6c8fd0a` en su rama de ledger, y `a0d710e2`…`3f15a02c` aquí), con F5 en
curso. Pendientes F6-F13 en el orden de la tabla.

## 8. Riesgos

- **F7/F8/F9 son diseño, no movimiento**: si el contrato no se fija antes de mover autoridad, se
  cambia comportamiento sin querer. Cada una exige decidir dueño del tick, del estado y de la
  publicación antes de tocar código.
- **Trocear no siempre acelera el build**: más unidades = más metadata y linking. El argumento del
  programa es navegabilidad y fronteras, y el build se mide aparte.
- **Deriva de la política**: los techos pueden quedar obsoletos si se suben para "hacer pasar" una
  fase. Este plan lo prohíbe explícitamente.
