# Estándar de estructura y convenciones de RustyCore

**Ámbito:** todo el workspace (crates existentes, refactors y desarrollo nuevo). Es el estándar
estructural del proyecto: si un trabajo contradice este documento, o se cambia el documento con
una razón, o se cambia el trabajo.

**Autoridad y separación de planos (regla central):**

- La **lógica y el comportamiento** tienen su autoridad en la referencia C++ versionada
  (`/home/server/woltk-trinity-legacy`, 3.4.3) y, como fuente complementaria aprobada, en
  AzerothCore. De ahí salen los bytes de paquete, el orden de fases, las fórmulas, la admisión, la
  persistencia y los ciclos de vida. Cada regla cita su ruta y su línea.
- La **estructura** (crates, capas, módulos, visibilidad, tests) **no** se copia de C++. Un
  proyecto C++ organiza su `game/` en directorios porque es C++ y un solo binario; nosotros
  decidimos la forma en Rust. Que en C++ exista `Maps/` o `Spells/` **no** obliga a un crate
  `wow-spells` ni al revés: inspira nombres y alcance de dominio, y nada más.
- Consecuencia práctica: **ninguna decisión estructural se justifica con "en C++ es así"**. Se
  justifica con este estándar y con la evidencia del propio repositorio.

## 1. Principios

1. **Capas acíclicas, flechas hacia abajo.** El compilador es la cerca: un crate no puede depender
   de quien depende de él.
2. **La aplicación es delgada.** El conocimiento (reglas) vive abajo; arriba solo orquestación.
3. **Un dueño por estado y por transición.** Sin espejos, sin contextos universales, sin un
   segundo lock para poder mover código.
4. **Las fronteras son estrechas:** el dominio recibe datos y devuelve una **intención**; la
   aplicación la aplica. Nunca cruza un `&mut` del tipo de sesión.
5. **Módulo antes que crate.** Un crate se gana con contrato propio, dirección forzada por el
   compilador y (≥2 consumidores o aislamiento de compilación útil).
6. **Tamaño acotado.** Los techos son una restricción de trabajo, no estética: un humano o un
   agente que debe cargar 100 k líneas para tocar una regla se bloquea.
7. **Movimiento y comportamiento, siempre separados.** Un arreglo de gameplay va en su commit, con
   su ancla y su prueba; nunca escondido dentro de una mudanza.
8. **Evidencia escalonada:** compila → suite del crate → composición → validación final única.

## 2. Capas y dirección de dependencias

| capa | crates | puede depender de |
|---|---|---|
| 0 base | `wow-core`, `wow-constants`, `wow-config`, `wow-crypto`, `wow-logging`, `wow-math`, `wow-proto`, `wow-collections` (utilidad) | 0 |
| 1 datos/durables | `wow-data` (solo catálogos), `wow-persistence`, `wow-database` | 0 |
| 2 modelo/estado | `wow-entities`, `wow-map`, `wow-movement`, `wow-packet`, `wow-network` | 0-1 |
| 3 reglas de dominio | `wow-<dominio>`: `combat`, `conditions`, `spell`, `spell-acquisition`, `loot`, `quest`, `items`, `pets`, `social`, `economy`, `instances`, `battlegrounds`, `dungeon-finding`, `progression`, `ai`, `account-collections`, `anticheat` | 0-2 (nunca la capa 4) |
| 4 aplicación | `wow-world` | 0-3 |
| 5 composición | `world-server`, `bnet-server`, `world-modules` | 0-4 |

Prohibido: dependencias hacia arriba, ciclos, y que un crate de dominio conozca `WorldSession`,
el transporte o la base de datos. Tipos compartidos: se van a su **dueño**; solo el vocabulario
genuinamente universal va a `wow-core`.

## 3. Nombres

- Crates: `wow-<concepto>` (base/modelo) y `wow-<dominio>` (reglas). Binarios: `world-server`,
  `bnet-server`. Solo-test: `wow-test-support`. E2E: `wow-e2e-tests`.
- Módulos: **el nombre dice la responsabilidad**. Prohibido `misc`, `utils`, `common`, `helpers`,
  `shared` como vertedero. Si un módulo no tiene dueño semántico, no debe existir.
- Los nombres de dominio se inspiran en AzerothCore cuando encajan (`combat`, `spells`, `loot`,
  `quests`, `groups`, `guilds`, `chat`, `auction`, `trade`, `mail`, `battlegrounds`,
  `dungeon-finding`, `instances`, `movement`, `conditions`, `reputation`, `profession`, `trainer`,
  `pets`), y se adaptan cuando el dominio real difiere. Inspiración, no obligación.

### 3.1 Dentro de un crate de dominio

```
wow-<dominio>/src/
├─ model.rs     datos del dominio (no conoce Session)
├─ rules/       funciones puras: entradas → decisión o intención
├─ state.rs     solo si el dominio posee estado mutable (con su dueño explícito)
├─ adapter.rs   puente de importación del consumidor (temporal; se retira al migrar)
└─ tests/       escenarios focales, junto a la regla que prueban
```

### 3.2 Dentro de la aplicación (`wow-world`)

```
wow-world/src/
├─ session/
│  ├─ state/         WorldSession + sub-estados dueños (transporte, acceso a Player,
│  │                 loot, hechizos, fence de persistencia, catálogos)
│  ├─ construction.rs · admission · lifecycle/ · driver/ · persistence/ · publication/
│  ├─ directory.rs · registry.rs   (registro único de opcodes)
│  ├─ handlers/<dominio>/          (solo adaptadores: decode → dominio → aplicar → publicar)
│  └─ tests/                       (escenarios de integración + fixtures)
└─ runtime/                        (orquestación de ticks; el estado de mapa vive en wow-map)
```

### 3.3 Nomenclatura de paridad (decisión del usuario, 2026-09-24)

El sufijo `_like_cpp` y los tipos `...LikeCpp` son **legado**: se usaron para marcar la procedencia
mientras se portaba. No aportan valor y **no se usan en código nuevo ni en refactors**: el
identificador nuevo lleva el nombre del dominio.

- La procedencia C++ se conserva donde sí vale: comentario de la regla, ancla
  `Fichero.cpp:línea`, mensaje de commit y ADR/checkpoint. **La paridad es de lógica; el nombre no
  es la paridad.**
- **No se renombra en masa el legado** dentro de este programa: son miles de identificadores, y
  cambiarían todas las baselines, firmas registradas y llamadores. El coste y el riesgo no
  compensan; si algún día se quiere, será una campaña propia con su aceptación.
- **Renombrado oportunista sí**: cuando un dominio se extrae a su crate o un fichero se reescribe
  de verdad, sus identificadores nuevos se ponen limpios. Si hace falta compatibilidad temporal, se
  deja un `adapter.rs` o un alias `pub use` con el nombre viejo, marcado para retirarse.
- El mismo criterio aplica a los tests: no se añade `_like_cpp` a tests nuevos, y los existentes no
  se renombran por estética.

## 4. Cómo se corta

1. **Clasificar la hoja**: *regla pura* (no menciona el tipo de sesión, no hace I/O) → se va al
   crate de dominio con sus tests · *estado* → se queda detrás de contrato estrecho · *orquestación*
   (vida, driver, persistencia, publicación) → se queda en la aplicación.
2. **Una operación completa por corte**, con su ancla C++ ya documentada; no rangos de líneas.
3. **Si el dominio necesita escribir**, devuelve un enum de intención y la aplicación lo aplica.
4. **Nada de traits por helper, campos públicos, espejos de estado ni locks nuevos** para poder
   mover código.

## 5. Visibilidad

Escalera, de más estrecha a más ancha: `private` → `pub(super)` → **`pub(in crate::<owner>)`** →
`pub(crate)` → `pub`.

- El ancho por defecto al mover código es **`pub(in crate::<owner>)`**, que conserva el ámbito
  efectivo anterior.
- `pub(crate)` solo cuando otra rama hermana del crate lo necesita de verdad.
- `pub` solo para la API real del crate (consumidores de otra capa o tests de integración).
- **Ensanchar visibilidad para que compile un movimiento es señal de corte mal hecho**, no una
  solución. Si hace falta, se replantea el corte.

## 6. Tests y fixtures

| ¿qué prueba? | dónde va |
|---|---|
| regla interna, necesita privados | `#[cfg(test)]` dentro del crate de dominio |
| contrato público de un dominio | `<crate>/tests/` |
| composición: bytes de paquete, orden, admissión de fase, COMMIT/persistencia, login/logout | `wow-world/tests/` o `wow-e2e-tests` |
| fixtures y builders compartidos por varios crates | `wow-test-support` (dev-dependency) |

Invariantes que **toda** mudanza de tests debe verificar (contadas antes y después):
`#[test]`/`#[tokio::test]`, `inventory::submit!` (registros de opcode) y la lista de escenarios.
Compilar no basta: un test movido puede compilar y no ejecutarse si se pierde del árbol.

## 7. Presupuestos

| unidad | límite | objetivo |
|---|---:|---:|
| fichero físico | 1 000 líneas | 400-600 |
| owner lógico (ratchet de hotspots) | 20 000 | — |
| crate de dominio | ~60 000 | — |
| capa de aplicación (`wow-world`) | 200 000 | — |
| módulo sin dueño (`misc`/`utils`/`common`) | 0 | 0 |

Los techos se actualizan solo con **delta revisado** tras una retirada validada; **nunca** se
regeneran a ciegas ni se suben para que pase una fase.

## 8. Árbol de decisión para código nuevo

1. ¿Regla determinista, sin sesión ni I/O? → crate de dominio, en `rules/`.
2. ¿Estado canónico mutable? → al dueño existente, con API estrecha.
3. ¿Bytes de red? → `wow-packet`/`wow-network`. ¿Durabilidad? → `wow-persistence`/`wow-database`.
4. ¿Orquestación de la sesión? → aplicación.
5. ¿Bootstrap/wiring? → composición.
6. ¿Sin contrato y un solo consumidor? → módulo privado, no crate.

## 9. Checklist de un cambio (refactor o desarrollo nuevo)

- [ ] El corte es **una operación completa** con su ancla C++ citada (o la referencia aprobada).
- [ ] La dirección de dependencias sigue bajando; no hay ciclo ni back-edge nuevo.
- [ ] Un solo dueño y escritor por estado/transición; sin espejos ni locks nuevos.
- [ ] Sin `&mut` del tipo de sesión cruzando a un dominio; entrada por DTO, salida por intención.
- [ ] Visibilidad: ámbito mínimo suficiente; ningún ensanchamiento "para que compile".
- [ ] Tests: colocados según §6; escenarios y assertions preservados; invariantes contados.
- [ ] Presupuestos de §7 respetados (fichero, owner, crate).
- [ ] Movimiento y comportamiento en commits separados.
- [ ] Ledger y política físicas con delta revisado; sin regeneración a ciegas.
- [ ] Evidencia reportada como tal: compilación ≠ suite ≠ composición ≠ validación final.

## 10. Aplicación mecánica (el estándar no se confía a la memoria)

- `[workspace.lints]` (+ política por crate) aplica clippy/rustc y la política de `unsafe` y de
  documentación de API pública.
- `cargo xtask check-layers` verifica las **aristas permitidas entre capas** (sin esto, una
  inversión puede volver sin que nadie lo note).
- `cargo xtask structure-audit` mide líneas, fichero mayor, stubs y crates sin consumidor.
- `cargo-machete` y `cargo-deny` vigilan dependencias no usadas, duplicadas o con avisos.
- **Features**: el mapa de subsistemas opcionales se decide una vez y se documenta aquí; el build
  base no puede requerirlos.
- Código vendido (por ejemplo el port de navmesh) se marca como exento de presupuestos y lints.

Lo que no esté en esos comandos no es estándar, es costumbre.

## 11. Cómo se cambia este estándar
Con un commit de documentación que explique la razón y el impacto, actualizando también
[wow-world-distribution-plan.md](wow-world-distribution-plan.md) si cambian fases o presupuestos y
[workspace-structure-programme.md](workspace-structure-programme.md) si cambia el orden o el estado
de las fases. No se crean documentos competidores: este es el estándar estructural, el plan de
distribución es su programa para `wow-world` y el programa maestro es el índice y el orden de todo
el workspace.
