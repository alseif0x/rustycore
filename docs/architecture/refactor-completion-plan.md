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

#### Entrega local aceptada — 9e6767bb

El contrato entregado conserva `GroupRegistry` como autoridad única y define la
convergencia del proyección `Player::m_group`: toda transición se aplica al miembro
o queda registrada para reconciliación. No hay segundo owner mutable, campo de
Session, lock, task, reloj ni variante de `SessionCommand` nuevos.

- Autoridad: `crates/wow-social/src/group/membership.rs` publica
  `member_group_state_like_cpp` (la membresía que el miembro debe converger) y
  `group_category_like_cpp` (si el grupo sobrevive), que distingue `Group::Disband`
  de `Group::RemoveMember` para un miembro cuyo aviso se perdió.
- Frontera de entrega: `crates/wow-world/src/session/directory/group_state.rs`.
  `deliver_group_state_command_like_cpp` encola y, ante `Full`, `Disconnected` o
  `StaleRegistration`, registra la obligación. La marca se indexa por GUID —no por
  encarnación— para que un aviso perdido por un reemplazo alcance al sucesor, y se
  olvida al desregistrar la sesión. No contiene membresía, líder, rol ni subgrupo y
  no responde a ninguna lectura de gameplay.
- Ejecución: `WorldSession::reconcile_group_state_like_cpp`
  (`session/social/group.rs`) converge en la fase `ReconcileGroupState` del driver,
  después del drenaje del buzón. Cubre membresía, subgrupo y las tres preferencias
  de dificultad del grupo, y deriva el paquete de teardown de si el grupo sobrevivió.
  Un comando que llegue después encuentra la proyección ya convergida y su propia
  guarda de grupo no publica dos veces.
- Comandos con estado enrutados por esa frontera: instalación de membresía
  (`ops_1.rs`), kick y disband (`ops_1.rs`), cambio y permuta de subgrupo
  (`ops_2.rs`) y dificultad (`session/instances/difficulty.rs`). Un comando
  descartado por su fase de admisión o por su guarda de orden difiere en vez de
  desaparecer; una eliminación que compite con un re-join al mismo grupo ya no
  revoca una membresía que la autoridad mantiene.
- Lectores corregidos: tap de criatura (`session/social/group.rs`) y propiedad de
  instancia (`session/instances/instance.rs`) resuelven por la autoridad, como C++
  al desreferenciar `Player::m_group`. Los lectores que ya revalidaban pertenencia
  quedan sin cambios.
- Movimiento estructural separado: la aplicación de comandos de grupo salió de
  `handlers/loot/handlers.rs` —que no es su responsabilidad y excedía su presupuesto
  físico— a `handlers/group/commands.rs`, método a método. El techo de ese fichero
  se ajusta a su tamaño reducido (2.463 → 2.365).

Aceptación local ejecutada en este árbol (host aarch64 de desarrollo):
`cargo test -p wow-world --lib` 3.841 tests, `cargo test -p wow-social --lib` 80
tests, y los tres objetivos de integración ligados a producción de `wow-world`
(`production_handler_registry_contract`, `production_login_player_owner`,
`production_character_rename`) en verde. `session-ownership-check check
--syntax-only`, `check_architecture.py check` y `self-test`, `cargo fmt --all --
--check` y `git diff --check` pasan; los deltas de inventario revisados son
exactamente la superficie nueva y la reubicación.

Límite conservado: no hay evidencia de runtime vivo, captura ni DB/reinicio/relogin
para esta entrega. La saturación se ejerce sobre el canal acotado real en la
composición de sesión, no sobre un servidor en ejecución.

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

#### Entrega local aceptada — #735

El residuo exacto de #735 queda retirado: `session/progression/reputation.rs` ya no
reconstruye un `ReputationMgrLikeCpp` ni escribe el agregado a través de
`gameplay_state_mut()`. No quedan llamadas a `from_player_gameplay_state_like_cpp`
ni a `write_to_player_gameplay_state_like_cpp`; ambas desaparecen del árbol.

- Estado y sus invariantes en el Player: `crates/wow-entities/src/player/reputation.rs`
  define `PlayerReputationStateLikeCpp` —el `Player::m_reputationMgr` de C++
  (`Player.h:3116`, construido en `Player.cpp:338`)— con el almacenamiento de
  `FactionState` indexado por `ReputationListID` (`ReputationMgr.h:63`), el recorte de
  standing a `Reputation_Cap`/`Reputation_Bottom`, contadores de rango que no
  desbordan, reacciones forzadas y `_sendFactionIncreased`. `PlayerGameplayState`
  sustituye sus cuatro campos sueltos por ese propietario único.
- Reglas con catálogos fuera de la frontera de entidades: `ReputationMgrLikeCpp` pasa
  a ser un préstamo de ese estado (`ReputationMgrRefLikeCpp` para lectura,
  `ReputationMgrMutLikeCpp` para transición). Resolución de facciones, rangos,
  spillover, paragón/renombre y construcción de paquetes siguen en `wow-world`.
- Tipos de valor: `ReputationRankLikeCpp`, `ReputationFlagsLikeCpp`, los umbrales y
  los topes se mueven a `wow-constants`, que `wow-entities` ya puede usar;
  `wow_data::reputation` los reexporta, así que los consumidores de catálogo no
  cambian. No se añadió ninguna dependencia prohibida ni un crate por helper.
- Consumidores migrados: carga/hidratación, publicación de login, banderas de
  reputación, recompensas de misión, efectos de hechizo, recompensas por muerte,
  proyección de guardado y save ack. Las banderas `need_send` y `need_save` siguen
  siendo independientes y el orden de publicación se conserva.
- Mejora observable registrada: la proyección de guardado ya no puede quedar
  ensombrecida por una fila duplicada del mismo `ReputationListID`, porque el Player
  guarda la forma de `FactionStateList` de C++. La prueba que fijaba el comportamiento
  del vector malformado se sustituye por la que fija esa invariante.

Aceptación local ejecutada en este árbol: pruebas de `wow-entities` (735),
`wow-world` (3.842) y el resto del workspace, más los controles de arquitectura y la
validación final registrados en la issue y en `STATE.md`. No se ejecuta QA viva ni
evidencia de DB/reinicio para esta entrega.

#### Entrega local aceptada — #752

Tercera macro P2 seleccionada por evidencia tras #735: el runtime de talentos tenía
la misma forma —campos públicos y un cierre genérico que entregaba `&mut` al
llamador— con doce consumidores de producción.

- Estado y sus invariantes en el Player:
  `crates/wow-entities/src/player/talent_runtime.rs` posee
  `PlayerTalentRuntimeState` con campos privados y las transiciones que C++ hace
  sobre `_talents`/`_specializationInfo`: `Player::AddTalent` (`Player.cpp:2644`),
  `SetGlyph` (`:25477`), la toma de grupo de `ActivateTalentGroup` (`:26894`),
  `ResetTalents` (`:3505`) y las banderas de hidratación de `_LoadTalents`
  (`:26623`) y `_LoadGlyphs` (`:26573`). Cada operación acota su índice de grupo y
  su ranura de glifo; un grupo fuera de rango se rechaza en vez de entrar en pánico
  y el grupo activo se recorta a la última especialización.
- Frontera retirada: `mutate_player_talent_runtime_like_cpp` deja de ser alcanzable
  desde otros módulos. `session/persistence/load.rs` y `session/persistence/commit.rs`
  llaman transiciones con nombre (instalar una fila de talento cargada, instalar un
  glifo cargado, marcar talentos/glifos cargados, instalar los grupos que deja un
  reset). El cierre queda privado al módulo propietario, con un gancho `cfg(test)`
  para las regresiones de despacho activo/detached/reemplazo.
- Reglas que no se movieron: catálogos, efectos de hechizo y aura, derivación de
  puntos de talento y construcción de paquetes siguen en `wow-world`.
- Límite registrado: C++ guarda `{State, Rank}` por fila de talento y la
  representación Rust solo guarda el rango. Esa diferencia es una frontera
  representada previa; esta entrega no la introduce ni la cierra.

Aceptación local: `wow-entities` (745) y `wow-world` (3.842) en verde, once
regresiones nuevas de invariantes, controles de arquitectura y ownership con delta
de inventario revisado. Sin QA viva ni evidencia de DB/reinicio.

#### Entrega local aceptada — #754

Cuarta macro P2 por evidencia: el runtime de hechizos tenía dieciséis campos
públicos y un cierre genérico con treinta y seis sitios de escritura de
producción repartidos por spellbook, adquisición, efectos, carga de persistencia,
aprendizaje por efecto y el reset de talentos.

- Estado e invariantes en el Player:
  `crates/wow-entities/src/player/spell_runtime.rs` posee
  `PlayerSpellRuntimeState` con los campos cerrados al módulo Player y las
  transiciones que C++ hace sobre `m_spells` (`Player.h:2961`) y
  `m_overrideSpells` (`:2962`). El propietario impone ahora lo que antes escribía
  cada llamador a mano: un hechizo dependiente nunca se registra como eliminado,
  la entrada de override se borra con su último reemplazo, marcar dependiente
  solo alcanza filas autoritativas y el rebase tras guardar recalcula cada
  conjunto derivado a partir de las filas que sobrevivieron.
- Banderas de completitud: conservan su significado —vacío autoritativo frente a
  propietario no hidratado— y siguen siendo hechos independientes, de modo que un
  consumidor falle cerrado ante cualquier combinación.
- Frontera retirada: `mutate_player_spell_runtime_like_cpp` queda acotado a
  `session::spell_state`. Autoridad de carga, reset de login, aprendizaje por
  efecto y reset de talentos usan transiciones con nombre; queda un gancho
  `cfg(test)` para las regresiones de despacho.
- Reglas que no se movieron: catálogos, reglas de rango/habilidad, planificador de
  adquisición y construcción de paquetes siguen en `wow-world`.

Aceptación local: `wow-entities` (760) y `wow-world` (3.842) en verde, quince
regresiones nuevas de invariantes, controles de arquitectura y ownership con
delta de inventario revisado. Sin QA viva ni evidencia de DB/reinicio.

#### Entrega local aceptada — #756

Quinta macro P2: el estado de misiones tenía catorce campos públicos y un cierre
genérico con treinta y ocho sitios de escritura de producción.

- Estado e invariantes en el Player:
  `crates/wow-entities/src/player/quest_state.rs` posee
  `PlayerQuestGameplayState` con los campos cerrados al módulo Player y las
  transiciones que C++ hace sobre `m_QuestStatus` (`Player.h:2947`),
  `m_RewardedQuests` (`:2951`), `m_DFQuests` (`:2481`) y `m_seasonalquests`
  (`:2838`). El propietario impone ahora el contador de objetivo que se inserta
  una vez y luego se actualiza, el reseteo estacional que solo elimina misiones
  completadas antes del nuevo inicio y borra el evento con su última misión, y la
  separación entre el conjunto recompensado en memoria y las filas persistidas.
- Bandera de autoridad: conserva su significado —carga vacía autoritativa frente
  a propietario no hidratado— y sigue siendo un hecho independiente.
- Catorce llamadores de una sola transición pasan a operaciones de sesión con
  nombre en `handlers/quest/{handlers,rewards,state,persistence}.rs`,
  `session/spell_effects/effects_progress.rs` y `session/mod.rs`.
- Proyección retenida y registrada: los cinco recorridos de objetivos de
  `handlers/quest/objectives.rs` y la compactación de huecos de
  `handlers/quest/state.rs` conservan el préstamo porque sus reglas necesitan
  plantillas y objetivos de misión que no pueden entrar en `wow-entities`, y C++
  las aplica sosteniendo el Player (`AdjustQuestObjectiveProgress`,
  `Player.cpp:15874`). **Condición de salida:** se retiran con el contrato de la
  operación de progreso de objetivos de #41.
- Se preserva la transacción representada de recompensa de #718.
- Distinción registrada: completar una misión estacional marca
  `seasonal_quest_changed` como C++ `SetSeasonalQuestStatus` (`Player.cpp:24067`),
  mientras que sembrar una fila almacenada no debe marcarlo o el siguiente guardado
  trataría estado intacto como sucio. Son dos operaciones con nombre y una regresión
  las fija.
- Hallazgo registrado, no reparado aquí: el vector `objective_progress` del estado de
  misiones no tiene lector ni escritor de producción; el progreso representado circula
  por `objective_counts` de cada estado y por la cola de eventos de la sesión. Se
  conserva el campo y su lectura; darle escritores pertenece a la operación de
  progreso de objetivos de #41.

#### Entrega local aceptada — #763

Sexta macro P2: el estado de colecciones tenía ocho campos públicos y un cierre
genérico con veinte sitios de escritura de producción.

- Estado e invariantes en el Player:
  `crates/wow-entities/src/player/collection_state.rs` posee
  `PlayerCollectionStateLikeCpp` con los campos cerrados al módulo Player y las
  transiciones que C++ hace en `CollectionMgr`
  (`Entities/Player/CollectionMgr.cpp`): `AddToy` (`:102`) sobre
  `UpdateAccountToys` (`:140`), `ToySetFavorite` (`:145`), `ToyClearFanfare`
  (`:157`), `AddHeirloom` (`:237`) sobre `UpdateAccountHeirlooms` (`:217`),
  `UpgradeHeirloom` (`:243`), `CheckHeirloomUpgrades` (`:278`), `AddMount`
  (`:360`), `MountSetFavorite` (`:395`), los cargadores de cuenta (`:113`,
  `:174`, `:330`, `:461`, `:874`), `SaveAccountItemAppearances` (`:516`),
  `AddItemAppearance` (`:732`), `AddTemporaryAppearance` (`:768`),
  `RemoveTemporaryAppearance` (`:777`) y `SetAppearanceIsFavorite` (`:828`).
- El propietario impone ahora lo que los llamadores escribían a mano: un juguete,
  reliquia o montura ya coleccionado no se sobrescribe —`_mounts.insert` de C++
  tampoco lo hace—, una apariencia permanente elimina los proveedores temporales
  que sustituye, una apariencia temporal solo se borra con su último proveedor, y
  el guardado de favoritas pasa `New` a `Unchanged` y borra `Removed` en una sola
  transición que devuelve sus planes de inserción y borrado.
- Los consumidores de producción en
  `session/{lifecycle_ops,player_items,persistence,movement,spell_state}` y la
  raíz de sesión llaman cada uno a una transición con nombre. El adaptador
  interno `mutate_player_collection_state_like_cpp` y su lectura conservan el
  ciclo instantánea/escritura sobre acceso canónico, pero pasan de `pub(crate)` a
  `pub(in crate::session)` y ya no entregan campos al llamador.
- Departuras registradas, no introducidas aquí: C++ posee las colecciones en la
  sesión (`WorldSession::_collectionMgr`, `WorldSession.h:1938`) mientras
  RustyCore guarda el estado representado en el Player, y el orden del guardado de
  favoritas es determinista por id de apariencia donde C++ recorre un mapa sin
  orden. Ambas preceden a esta entrega.
- Sin API especulativa: las operaciones que solo tenían consumidores de fixture se
  retiraron y esos fixtures usan las instalaciones de producción.

Aceptación local: diecisiete regresiones nuevas de invariantes en
`player_tests/collection_state.rs`, controles de arquitectura y ownership con
delta de inventario revisado (session/mod.rs -68 producción/+7 test;
player/mod.rs +335 producción/+278 test). Sin QA viva ni evidencia de
DB/reinicio/relogin.

#### Entrega local aceptada — #765

Séptima macro P2: el estado de taxi tenía ocho campos públicos y un cierre
genérico con cinco sitios de escritura.

- Estado e invariantes en el Player:
  `crates/wow-entities/src/player/taxi_state.rs` posee `PlayerTaxiState` y sus
  dos registros de vuelo con los campos cerrados al módulo Player y las
  transiciones que C++ hace sobre `PlayerTaxi`
  (`Entities/Player/PlayerTaxi.h`): `IsTaximaskNodeKnown` (`:44`),
  `SetTaximaskNode` (`:50`), la instalación de ruta tras
  `LoadTaxiDestinationsFromString` (`:65`), `GetTaxiDestination` (`:70`)
  derivada de la cola como la deriva C++, y la limpieza de aterrizaje
  `Player::CleanupAfterTaxiFlight` (`Player.cpp:22019`).
- La limpieza deja de ser cuatro escrituras de campo en el cierre del llamador:
  vaciar la ruta, desmontar y quitar `UNIT_FLAG_REMOVE_CLIENT_CONTROL |
  UNIT_FLAG_ON_TAXI` son una sola transición, y el avance de vuelo tras el
  teleport tampoco puede aplicarse a medias.
- Registrado, no inventado: `source_node_id` y `destination_node_id` no tenían
  lector ni escritor de producción y se retiran, porque C++ los deriva de
  `m_TaxiDestinations`. La máscara de nodos conserva sus operaciones C++ pero
  sigue sin llamador de producción. **Condición de salida:** la operación de
  conocimiento de nodos de taxi que los aprende y publica.
- No se modelan en el propietario `AddTaxiDestination` (`:68`) ni
  `NextTaxiDestination` (`:74`): la ruta la construye y consume la ruta de vuelo
  dirigida por catálogo en `wow-world`.
- Departuras conservadas y nombradas en el módulo: C++ no guarda registro de
  vuelo en `PlayerTaxi` y mantiene las banderas de unidad en `Unit`; RustyCore
  refleja ambas junto a la ruta para que la limpieza siga siendo una transición.

Aceptación local: catorce regresiones nuevas de invariantes en
`player_tests/taxi_state.rs`, controles de arquitectura y ownership con delta de
inventario revisado (session/mod.rs -2 producción/+1 test; player/mod.rs +200
producción/+188 test). Sin QA viva ni evidencia de DB/reinicio/relogin.

#### Entrega local aceptada — #767

Octava macro P2: los conjuntos de equipo vivían como dos miembros públicos
sueltos del estado de juego y un cierre genérico que entregaba `&mut BTreeMap`
y `&mut bool` al llamador.

- Estado e invariantes en el Player:
  `crates/wow-entities/src/player/equipment_sets.rs` posee
  `PlayerEquipmentSetsLikeCpp` —C++ `Player::_equipmentSets` (`Player.h:3050`)—
  con almacenamiento privado y las transiciones que C++ hace:
  `_LoadEquipmentSets` (`Player.cpp:16907`), `SetEquipmentSet` (`:26376`),
  `_SaveEquipmentSets` (`:26409`) y `DeleteEquipmentSet` (`:26524`).
- `SetEquipmentSet` se separa en las dos operaciones que C++ distingue por el
  guid de la petición: la creación, para la que C++ toma el guid de
  `GenerateEquipmentSetGuid`, y la edición, que rechaza un guid no almacenado
  exactamente como C++ registra y retorna, y aplica la regla de estado de
  `:26406` —lo que sigue siendo `New` se queda en `New`, lo demás pasa a
  `Changed`— para que un conjunto creado y editado antes del primer guardado se
  inserte una sola vez.
- El reconocimiento del guardado diferido deja de ser una función libre en
  `save_ack.rs` y pasa al propietario, documentado como reconciliación propia de
  RustyCore y no como función C++: C++ escribe sus sentencias dentro de
  `_SaveEquipmentSets` sosteniendo el Player y no tiene nada que reconciliar.
- Los diez consumidores de producción en
  `session/player_items/{equipment_sets,appearance}.rs`, la proyección de
  guardado del ciclo de vida y la raíz de sesión llaman cada uno a una
  transición con nombre. Los dos campos espejo `#[cfg(test)]` de la sesión se
  funden en uno del mismo tipo y el inventario de campos pierde uno.
- Corrección de comportamiento nombrada, no escondida: las filas se indexan por
  su propio guid, como C++ indexa `_equipmentSets`, donde antes los fixtures
  insertaban con una clave junto a una fila cuyo guid quedaba a cero. Cinco
  pruebas de `character_tests/item_1.rs` dependían de esa inconsistencia y
  siguen el guid real sin cambiar sus aserciones de estado.
- La validación de catálogo, la generación de guid y la construcción de paquetes
  siguen en `wow-world`, donde la política de dependencias las permite.

Aceptación local: dieciséis regresiones nuevas de invariantes en
`player_tests/equipment_sets.rs`, controles de arquitectura y ownership con
delta de inventario revisado (session/mod.rs -37 producción/+8 test;
player/mod.rs +177 producción/+307 test) y baseline de ownership actualizado por
el campo fundido y las dos firmas del préstamo. Sin QA viva ni evidencia de
DB/reinicio/relogin.

#### Entrega local aceptada — #769

Novena macro P2: el runtime de modificadores de objeto tenía tres miembros
públicos y un cierre genérico con seis sitios de escritura.

- Estado e invariantes en el Player:
  `crates/wow-entities/src/player/item_modifiers.rs` posee
  `PlayerItemModifierRuntimeStateLikeCpp` con los miembros cerrados al módulo
  Player y las transiciones que C++ hace sobre `Player::ItemSetEff`
  —`ItemSetEffect` en `Entities/Item/Item.h:41`— mediante `AddItemsSetItem`
  (`Entities/Item/Item.cpp:57`) y `RemoveItemsSetItem` (`:146`), con el borrado
  del efecto cuando se va su última pieza equipada (`:192`) como cola propia,
  porque la retirada de bonus intermedia necesita las filas de catálogo.
- El propietario impone ahora lo que escribían los seis llamadores: el efecto se
  crea con la primera pieza, un bonus solo lo sostiene un efecto existente,
  quitar una pieza de un conjunto sin efecto es el retorno temprano de C++ y no
  una inserción, y un efecto borrado se lleva sus bonus restantes.
- Los tres campos espejo `#[cfg(test)]` de la sesión se funden en uno del mismo
  tipo; el inventario de campos pierde dos.
- Proyección retenida y registrada: `with_bonuses_mut_like_cpp` presta el
  registro de bonus a las reglas de encantamiento y equipo de
  `session_rules/rules_1.rs`, que recorren acciones con forma de catálogo que no
  pueden entrar en `wow-entities` y que C++ aplica sosteniendo el Player
  (`Player::_ApplyItemBonuses`, `Player::ApplyEnchantment`). **Condición de
  salida:** que el contrato de aplicación de encantamiento/equipo pase a una
  operación con nombre que reciba el efecto resuelto en lugar del registro.

Aceptación local: catorce regresiones nuevas de invariantes en
`player_tests/item_modifiers.rs`, controles de arquitectura y ownership con
delta de inventario revisado (player/mod.rs +236 producción/+192 test;
session/mod.rs -25 producción/-4 test) y baseline de ownership actualizado por
los campos fundidos. Sin QA viva ni evidencia de DB/reinicio/relogin.

#### Entrega local aceptada — #771

Décima macro P2: la petición de lanzamiento en cola era un campo público y un
cierre genérico que entregaba `&mut Option<...>` al llamador.

- Estado e invariantes en el Player: `pending_spell_cast` se cierra a
  `wow-entities` y `crates/wow-entities/src/player/pending_spell_cast.rs` recibe
  las transiciones que C++ hace sobre `_pendingSpellCastRequest`
  (`Player.h:3154`): `RequestSpellCast` (`Player.cpp:29078`), que devuelve la
  petición a la que sustituye porque C++ la cancela en línea en `:29082`;
  `CancelPendingCastRequest` (`:29091`), que devuelve la petición cuyo cast id y
  spell id C++ lee para `CastFailed` en `:29098`; y la toma con identidad
  comprobada de `ExecutePendingSpellCastRequest` (`:29122`).
- La identidad es el cast id, el spell id y la unidad lanzadora, exactamente lo
  que C++ vuelve a leer de la petición antes de ejecutarla: una petición llegada
  después no la consume el plan anterior.
- El paquete `CastFailed` se queda en la sesión, que posee la conexión, así que
  cada operación devuelve la petición sobre la que el llamador debe informar en
  lugar de informar ella misma.
- Los tres consumidores de producción de `session/player_cast/state.rs` llaman a
  transiciones con nombre a través de un ayudante privado que aplica la
  operación canónica del Player o el espejo `#[cfg(test)]` sin handle.
- Una construcción literal `#[cfg(test)]` de `PlayerGameplayState` en la raíz de
  sesión pasa a un constructor con nombre para los campos de actualización del
  jugador activo, porque el campo cerrado invalidaba la actualización funcional.

Aceptación local: diez regresiones nuevas de invariantes en
`player_tests/pending_spell_cast.rs`, controles de arquitectura y ownership con
delta de inventario revisado (player/mod.rs +80 producción/+158 test;
session/mod.rs +30 producción/+4 test, crecimiento que es el camino dual
explícito del ayudante). Sin QA viva ni evidencia de DB/reinicio/relogin.

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
