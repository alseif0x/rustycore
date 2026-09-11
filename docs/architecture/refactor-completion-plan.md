# Plan técnico para completar la arquitectura de RustyCore

**Sincronización de la entrega #748 — 2026-09-11.** Este documento detalla los
límites técnicos de la dirección general que mantienen `docs/migration/PORT_PLAN.md`
y GitHub #49. No es un plan de issues alternativo: el índice macro, sus lanes y sus
dependencias viven en el plan de port; aquí se fijan propietario, consumidores,
anclas C++, orden de ejecución y criterios de aceptación de la arquitectura.

El alcance de esta sincronización es documental. No reabre el análisis completo del
port, no inventa nuevas microissues y no convierte una prueba histórica en evidencia
nueva. Cada macro incluye sus consumidores y sus pruebas; la validación final sigue
la cadencia de `AGENTS.md`.

## 1. Estado que gobierna el plan

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

## 2. Evidencia y límites actuales

La evidencia fechada debe conservar su SHA, tipo y límites:

| Evidencia | Estado que puede sostener |
| --- | --- |
| #718 / `78276ddf57463fc0f568c1c4bcf84d619af68cad` | La recompensa representada usa una transacción coherente de personaje y tiene contrato de COMMIT ambiguo. No hubo escritura real de DB, reinicio ni relogin QA. |
| #716 / `6ae62d73a9f910ebd664d82e331520b836a73c77` | El analizador y sus inventarios fueron aceptados localmente; los 363 tests del analizador son evidencia histórica, no una ejecución de #748. |
| Conformidad hecs V2 | Pasó dentro de los límites del laboratorio registrados en `modularity-conformance-results.md`. Producción no tiene instalado `hecs` ni Wasmtime en esta base. |
| Traza de relojes | La traza fechada de seis relojes orienta la investigación; no prueba un nuevo inventario exhaustivo ni doble tick. |
| Organización física | La medición fechada de 31 archivos por encima de 2.000 líneas y 63 por encima de 1.000 sigue siendo deuda P4. No es una medición nueva ni se deben refrescar techos para ocultarla. |

El contrato de la recompensa y sus participantes no implementados permanece en
[quest-reward-operation-contract.md](quest-reward-operation-contract.md). Sus anclas
Classic son `Player.cpp:14625` (`RewardQuest`), `SaveToDB(false):14867`, la
transacción de personaje en `SaveToDB:19312` y el correo separado en `14794`.
El cierre de #718 no demuestra durabilidad real ni completa el port de participantes
no representados.

## 3. Propiedad y estructura que deben conservarse

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
   [module-design-guidelines.md](module-design-guidelines.md). No se cambian aquí sus
   presupuestos.

La selección de `hecs` es privada y selectiva: no crea un ECS global, reemplaza el
scheduler, expone queries públicas ni descompone todo Player. La conformidad de
laboratorio ya pasada habilita seguir inspeccionando el límite; no autoriza instalar
dependencias de producción antes del gate de #584.

## 4. Continuación P0–P6 y secuencia técnica

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

### 4.1 #743 — entrega y reconciliación de comandos de grupo

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

### 4.2 #735 — encapsulación de reputación bajo Player

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

### 4.3 Residuales P2 y paso a P3/P4

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

## 5. Producto #583 y auditoría #153

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

## 6. Método de cada macro y aceptación

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

## 7. Registro historico conservado

El handoff de #716 se conserva como evidencia en [STATE.md](../migration/STATE.md) y
en [modularity-and-ecs-plan.md](modularity-and-ecs-plan.md), incluyendo el commit
`6ae62d73a9f910ebd664d82e331520b836a73c77`, los 65 bridges preservados, siete registros
recuperados y los 363 tests históricos del analizador. El modo físico terminal fallaba
en la medición fechada de 31 archivos; no se presenta como aceptación de #584.

La evidencia histórica de #718 conserva el contrato de
[quest-reward-operation-contract.md](quest-reward-operation-contract.md), el commit
`78276ddf57463fc0f568c1c4bcf84d619af68cad` y la ausencia explícita de DB/restart/relogin
QA real. Los checkpoints de #578, #585, #587, #588 y #589 siguen siendo fuentes de
alcance y captura para sus entregas cerradas; sus antiguas listas de "siguiente paso"
no son instrucciones actuales.

### Inventario físico histórico — revisión #716

La medición fechada de #716 registró estos 31 paths por encima de 2.000 líneas
físicas. Se conserva completa para trazabilidad; no es una medición nueva ni una
excepción terminal automática. La responsabilidad y la salida de cada path siguen
la política de [module-design-guidelines.md](module-design-guidelines.md).

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
