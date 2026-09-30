# B6: contrato de ejecución y retiro del bridge

Responsabilidad de #1233 bajo #584. Revisión de fuente del 2026-09-29 en
`584-map-manager-domain`, HEAD `d6c92488` más cambios locales sin validar.
El [programa vigente](workspace-structure-programme.md#estado-de-continuacion)
gobierna el estado y la aceptación; este documento fija el contrato de B6.
No acredita compilación, paridad ni cierre de la responsabilidad.

## Comportamiento y owners actuales

Referencia 3.4.3, checkout `a5f8da2e`:

- `World.cpp:2701-2704/2748`: sesiones antes del update de mapas.
- `MapManager.cpp:287-318`: admisión con un diff, unload, updates, espera y
  `DelayedUpdate`; `DestroyMap` ejecuta `UnloadAll` antes de eliminar.
- `Map.cpp:666-815/2191`: árbol dinámico, sesiones, respawns, ObjectUpdater,
  transportes, SendObjectUpdates y tail; DelayedUpdate es posterior.
- `GameEventMgr.cpp:1174/1246`: creación y retirada de criaturas de eventos
  afectan el mismo objeto de mapa, junto con su metadata de grid.
- `Object.cpp:1838/1956`: un summon cuya inserción falla no queda publicado.

Rust conserva dos managers y dos productores con intervalos/diffs propios.
`world-server/runtime/map/update_loop.rs` coordina el tick canónico y su barrera
de sesiones. La familia `runtime/delivery` ejecuta PlayerMelee, Lifecycle,
Movement, Aggro, Spell y Melee sobre actores legados. La copia canónica de
Creature no es el motor de esas transiciones: `ExternalRuntime` omite su
visitor, conservando los otros objetos y el tail canónico.

`wow-map/manager/map_update.rs` calcula el plan NearbyCells una sola vez.
Ese plan difiere de la enumeración de todos los GUIDs en grids cargados del
envelope legado. El mismatch de incarnation del bridge se diagnostica después
de sus efectos; no es una barrera que los impida. Ninguno de esos mecanismos
autoriza filtrar el motor legado por el envelope actual.

## Cobertura de lifecycle que el retiro debe resolver

| camino | evidencia actual y límite |
|---|---|
| Grid y spawn-group | `runtime/game_events/grid.rs::mirror_loaded_grid_creature_to_legacy_like_cpp` se invoca después de insertar canónicamente. `false` mezcla contraparte existente, ausencia del manager/key e inserción fallida; no prueba cobertura. |
| Event spawn/unspawn | Spawn usa el mirror anterior. `runtime/game_events/unspawn.rs::game_event_unspawn_object_guid_list_for_event_like_cpp` retira metadata/respawns y actores canónicos sin recibir el manager legado; puede dejar su actor activo. |
| Respawn legado | `session/legacy_runtime/creature_lifecycle_tick.rs` inserta legado primero. La inserción canónica posterior puede devolver `None` sin rollback; su contador aumenta incluso con `None`. |
| Registro Session | `session/world_entities/creature_registry.rs` intenta la inserción canónica y puede añadir legado aunque falle. Un `Some` también puede representar una Creature existente con autoridad reconciliada; no equivale sólo a una inserción nueva. |
| Remove/unload | Remove Session retira legado antes de una retirada canónica que puede no ejecutarse. La destrucción canónica no tiene limpieza del manager legado. |
| Summon temporal | No se localizó un camino productivo Rust equivalente al TempSummon C++; el handler battle-pet declara ese hueco. No presentar esa ausencia como cobertura completa. |

Estas diferencias preexistentes son trabajo explícito de consistencia de owners,
separado de la reubicación y del cambio de scheduling. No introducir una reparación
de gameplay encubierta ni retirar actores silenciosamente para superar una medida.
Antes de habilitar una selección canónica hay que resolver todos sus lectores,
escritores, publicación y resultados de inserción/retirada, por operación completa.

## Frontera interna fijada para la barrida

La primera implementación conserva el wrapper productivo por mapa y sus callers.
No altera los productores ni el flujo entre mapas. Divide su operación en:

1. Preparar: fijar el mismo diff y el plan NearbyCells/WholeTypedStores,
   actualizar DynamicObjects y muestrear `now_secs` en su punto actual.
2. Creature: ejecutar el visitor canónico o el skip ExternalRuntime actual,
   conservando el owner y el summary. No añadir un segundo writer.
3. Consumir la continuación: PvP refs, GameObject/pool, Transport, AreaTrigger,
   Conversation, SceneObject, SendObjectUpdates, scripts/weather/personal phase,
   move-lists, relocation y tail. Muestrear `now_ms` después de GameObject,
   en el punto actual; no adelantarlo a la preparación.

La continuación es privada, no Clone/Copy, contiene sólo diff, plan de objetos
y tiempo de esta operación. Se prepara una vez y se consume una vez. El wrapper
llama las tres fases inmediatamente y conserva la secuencia existente. No es
un contexto universal ni un nuevo API público sin consumidor.

El siguiente corte implementa la continuación del manager: consume el plan
AwaitingSessions y conserva Resuming durante toda la secuencia. Prepara un solo
mapa en cada paso, rechazando otra preparación y un cierre prematuro mientras
esa unidad esté pendiente. Cada token lleva epoch/incarnation y el plan privado;
finish verifica esa identidad antes de Creature y tail, y no modifica un mapa
recreado bajo la misma key. La composición actual consume las fases adyacentes
bajo su guard; todavía no ejecuta trabajo externo entre ellas. Los wrappers de
compatibilidad mantienen el orden por mapa, el accounting de MapUpdater y una
sola espera final antes de visibility/removal/DelayedUpdate. Soltar un token no
devuelve el manager a Idle ni finge haber terminado efectos inciertos.

Epoch e incarnation son contadores locales: pueden coincidir entre managers.
Plan, continuación y token retienen además una identidad privada de origen;
el manager la verifica antes de respawns/preparación, finish, finalize o abandon.
El witness sólo identifica al owner, sin otro estado mutable ni lock. Se escriben
casos con dos managers y contadores iguales para rechazar el consumo cruzado.

## Contrato del siguiente seam entre guards

La composición futura debe retener un único plan consumible por epoch y los
mapas/incarnations admitidos. Después de sesiones y respawns, preparará la
selección exacta y DynamicObjects. Ejecutará el motor Creature externo sólo
después de liberar los guards sincrónicos; completará las familias restantes y
SendObjectUpdates antes del barrier/DelayedUpdate. Los efectos se autorizan con
identidad/incarnation vigente antes de mutar o publicar, no después.

La espera externa no admite un tick nuevo, una segunda preparación ni un tail
ficticio. Un trabajo cuya cancelación no se conoce permanece pendiente bajo la
misma barrera; no devolver Idle mientras un closure puede seguir mutando.
Debe conservarse el fence que ordena mutación de respawn y envío a su mailbox,
sin guards de Map/entity durante await, I/O o entrega de paquetes.

El shutdown actual espera diez segundos por productor y diez por writer.
`stop_respawn_db_producer_like_cpp` aborta el task exterior al vencer su plazo;
eso no cancela un `spawn_blocking` ya iniciado. Cerrar el mailbox rechaza envíos
tardíos, pero no demuestra que el closure haya terminado. La unificación debe
resolver la vida de ese trabajo antes de declarar drenados sus owners; cualquier
fallo terminal se conserva como fallo, no como parada correcta.

## Evidencia terminal pendiente

**Preparación del traslado del actor (2026-09-30, sin aceptar):** implementada
la extracción de 24 campos runtime a WorldCreatureRuntime, un agregado privado
dentro de WorldCreature. Conserva motores, RNG, deadlines, procedencia y el contrato
manual de Clone: reconstrucción de MotionMaster y reset de chase/activo.
Se escriben dos casos de move y snapshot-clone, pendientes de ejecución.
No cambia aún el dueño ni retira la segunda copia. El destino propuesto es
una variante privada de EntityWorld que posea el WorldCreature real por move,
sin un MapObjectRecord duplicado. Antes de activarla deben migrarse el writer
de snapshots (`creature_canonical_adapter.rs:26`), las admisiones y todos los
ticks; el respawn legacy-first no puede convertirse en rollback implícito.

**Préstamos y valores propios (revisión 2026-09-30):** implementadas vistas
privadas ObjectRef/ObjectMut para los lectores y escritores de EntityWorld.
Conservan kind/cuerpo tipado y el recorrido actual, sin clones ni otro store.
Cinco casos de gates, reborrow y lifetime escritos, sin ejecutar. El manager
consume las proyecciones de lectura dentro del crate; no son exports públicos.
Ese cambio todavía no admite actores. ObjectAccessorMapSource conserva su
contrato propio en entidades: sólo se encontró TestMapSource como implementación,
sin un consumidor productivo de Map. No añadir nueve métodos públicos para
resolver un préstamo que los callers productivos no usan.

El siguiente traslado debe conservar también los valores retirados, no sólo
las vistas: `map/relocation.rs:463/524` retira y reinserta el cuerpo completo,
incluso para un movimiento dentro de la misma celda. Convertir un actor a
MapObjectRecord ahí perdería MotionMaster, RNG y deadlines. La entrada privada
propia debe viajar intacta por relocation y reinserción. Insert/displace y
remove/delete requieren contratos separados; un snapshot no equivale a crear
una nueva incarnation. `remove_from_map_like_cpp` actualmente borra el cuerpo
tipado incluso en su resultado no-delete y detacha loot: ese gap no se resuelve
silenciosamente cambiando el tipo de almacenamiento.

Implementado y revisado en fuente el transporte privado ObjectEntry, sólo Record:
dos retiradas y cuatro reinserciones de relocation mantienen el valor completo.
Siete casos escritos cubren identidad, autoridades, índices, desplazamiento,
admisión, relocation/fallback alcanzable y Player; no se han ejecutado.
Los retornos públicos existentes siguen siendo MapObjectRecord; su conversión
privada exhaustiva obliga a revisar el contrato al añadir el actor, sin un
fallback que descarte runtime. Los cuatro escritores externos de producción
descartan el desplazado; no se encontró retirada externa productiva. Antes de
activar una variante CreatureActor deben migrarse esos retornos y la operación
especial de snapshots, conservando timeline/revisión, loot CAS e índices.

Implementado y revisado el contrato OwnedMapObject opaco para insert/displace y remove: conserva
el valor completo sin Clone, proyección ni reinserción pública sin consumidor.
El inventario encontró siete lectores de contenido, todos tests internos; se
adaptarán con una proyección privada. replace_creature_snapshot recibe el Record
tipado validado y rechaza un destino que no sea Creature exacta antes de mutar.
El writer conserva su guard, CAS, health timeline y refs recíprocas. Diez casos
nuevos escritos para snapshots/validación y extracción, pendientes de ejecución.
into_creature mueve el cuerpo entero; no habilita aún el actor ni sus productores.

Implementada la variante privada CreatureActor(Box<WorldCreature>): un solo Creature,
sin Clone ni reconstrucción del motor. Validación prestada conserva el orden de
errores y devuelve el actor intacto si falla. Las vistas proyectan el cuerpo
prestado; snapshot sustituye Creature completo conservando Box, runtime y
create_data. OwnedMapObject lleva esa entrada por desplazamiento y relocation;
la eliminación consume el actor sólo en la disposición terminal existente.
Casos escritos deben sembrar motor, RNG y deadlines no predeterminados desde su
owner privado. Once casos cubren esa continuidad; la revisión corrigió la fixture
Point que sólo lanzaba un spline: ahora instala un generador real de prioridad
Highest antes de Chase. Sin ejecución; este almacenamiento no activa productores.

Implementada la operación completa WorldCreature::step_movement en
wow-map/src/map_manager/movement/step.rs (265 líneas), con dos callers
productivos conservados. Clock → spline → MotionMaster → home/random/waypoint/chase pasa al
actor; APP conserva callbacks perezosos de policy/path, MonsterMove/Stop y trace.
Nueve casos originales trasladados, seis APP conservados y diez nuevos de
contrato escritos. Fuente revisada: cuatro lecturas de filter, cuatro de owner
y dos clones de corredor redundantes ya existían; no se añadieron al trasladar.
El resultado transporta spline/stop sin segunda copia mutable ni nuevas deps de
Packet/Data/Config en Map. Anclas a5f8da2e Unit.cpp:479-480 y Map.cpp:666-807.
La operación no debe activarse bajo un guard canónico que bloquee al worker de
paths: la transferencia del motor no resuelve por sí sola esa frontera de I/O.
La integración posterior debe conservar NearbyCells y validar incarnation antes
de efectos/publicación, con una sola fuente de diff y barrera World/Map.

Revisión de consumidor 2026-09-30: world-server/runtime/map_tick.rs:101-161
todavía prepara y termina cada mapa bajo un guard, con ExternalRuntime que no
ejecuta criaturas. manager/tick_objects.rs:157-276 ya guarda selección, diff,
origen, epoch e incarnation, pero sólo valida al terminar. La integración debe
validar también antes de cada mutación y de consumir un resultado de path;
no basta añadir el bridge actual entre prepare/finish: reentra en el manager,
enumera todos los GUID y publica antes de comprobar incarnation.
session_factory.rs:170-238 mantiene las sesiones sobre la rail del productor;
shutdown puede iniciar otro drain. El corte de I/O necesita continuación real,
sin extraer provisionalmente el actor del store ni repetir draws/timers, y un
contrato de cancelación/settlement que no deje el manager en Resuming.
Estas condiciones son trabajo pendiente; no acreditan scheduler único.

Implementada y revisada en fuente la separación prepare/query/resume de Home/
Random/Waypoint/Chase, compartida por sus APIs síncronas actuales. Pending 124,
familias 113/153/153/270, terreno 202 y 15 casos nuevos en 450 líneas, sin ejecución.
No activa producción canónica. La revisión de movement/terrain.rs:70-117 encontró que normalizar Z
consulta static/grid height antes de leer capabilities; el servicio puede cargar
tiles del disco. Deben liberarse ambos I/O, no sólo Detour. Requests concretos,
pending owned sin Clone y terreno lazy deben conservar actual_end antes de los
puntos, los cinco draws Random y las mutaciones anteriores/posteriores al query.
El actor permanece en EntityWorld. Su identidad privada contra ABA, exclusión
de mutadores durante pending, cancelación/settlement y conexión del tick único
siguen pendientes; ninguna fixture de prepare/resume sustituye esa integración.

### Decisiones para integrar el actor y el productor (2026-09-30)

La identidad del actor debe pertenecer a la entrada privada de EntityWorld,
no al GUID, la dirección de Box ni la revisión de salud. Un witness inmutable
retenido por entrada y continuación permite distinguir una readmisión del mismo
GUID sin un segundo store, contador global o lock. Snapshot y relocation deben
conservarlo; una admisión nueva debe recibir otro. Resume requiere comprobar
origen del manager, epoch, incarnation del mapa y witness antes de tocar el actor.
Esto resuelve ABA; no sustituye la exclusión de escritores durante el I/O.

La exclusión debe aprovechar el productor y la rail existentes: ninguna fase
Session nueva entre preparación Creature y tail, y GameEvent después del tail
en ese mismo productor (`runtime/map/update_loop.rs:394-458`). El inventario
actual confirma que sus cambios de equipo/modelo y NPC flags reciben el manager
del consumidor (`runtime/game_events/live.rs:434/544`); no son un segundo loop.
Todavía hay que completar el inventario de escritores, incluidos finalizers,
admisión y retirada. No resolverlo con un guard Map retenido durante await,
mutaciones omitidas por un flag, otra copia de actor o reintentos del motor.

Shutdown es un escritor distinto: `app/runtime_supervision.rs:185-246` inicia
KickAll/flush mientras el productor sigue vivo; `session_factory.rs:210-238`
puede drenar controles independientemente de la rail. Antes de activar el seam,
la supervisión debe solicitar quiescencia al coordinador, impedir admisión de
otro tick y esperar el settlement del trabajo admitido antes de habilitar ese
drain. Conservar después KickAll → UpdateSessions(1) → StopNetwork y el fence
mutation-to-submit. Idle observado sin cerrar la admisión no es una barrera.
La barrera está implementada (quiescence 344, tests 361, 21 nuevos), sin aceptación;
revisadas en fuente admisión, tickets, continuaciones, finalización y supervisión.
El ledger debe usar el mutex existente de ActiveSessionRegistry: cerrar la admisión
de productores y registros es atómico, y ShutdownRequested no autoriza Drain.
Cada tick admitido conserva una obligación no Clone hasta completar realmente sus
efectos y submits. La obligación legacy vive dentro de spawn_blocking, no en el
task exterior abortable; la canónica incluye GameEvents posteriores al tail, sus
awaits, mutaciones y entregas. Un permit Session ya reclamado conserva la obligación
a través de la interrupción del loop; Idle, Drop, panic, handle ausente o timeout
no la completan ni producen un recibo válido de quiescencia.

Los finalizers también escriben. Un WorldFinalization ya admitido conserva su
permit hasta destruir Session y retirar el registro; espera el productor legacy
sin esperar su propio permit. La finalización fuera de fase retira readiness,
impide nuevos ticks y espera los admitidos antes de sus efectos; conserva esa
exclusión durante awaits y destrucción. Sólo éxito explícito puede reabrir una
exclusión temporal, y nunca tras ShutdownRequested. Intenciones concurrentes de
finalización no permiten intercalar un tick. El recibo de drain queda vinculado
al issuer y generación de cierre; el tick final de respawn requiere registros
vacíos y ninguna obligación de tick/finalizer incierta. Timeout mantiene fallo y
el intento de controles/network, sin fabricar autoridad de drain ni finalizar
owners mientras una closure puede seguir mutando. Esto no acredita settlement
de tareas independientes de quest ni autoriza runtime o DB. Un recibo terminal
distinto exige registros, ticks e intents/finalizadores vacíos; la supervisión
lo espera antes de autorizar respawn final y antes de cerrar el mailbox. El Drop
del registro puede despertar su espera mientras sigue vivo el lease del finalizer;
no confundir ese registro vacío con terminación. El owner conserva la secuencia
Drop Session → Drop registro → pending.complete → admission.complete.

Las consultas de ruta/terreno deben recibir exclusivamente datos owned de query,
sin acceso mutable al actor. Su terminación se distingue de la terminación del
tick; cancelar la espera no revierte las mutaciones ya realizadas por prepare.
La continuación debe tener disposición explícita antes de cerrar accounting o
permitir escritores de shutdown. Contraste C++: World.cpp:2701/2748 y
Map.cpp:666-710, checkout a5f8da2e; la concurrencia Rust exige esta frontera sin
añadir otra fuente de diff. Son decisiones de integración, no código aceptado.

Implementado y revisado el seam `runtime/map_tick/object_work.rs` (117 líneas):
trabajo owned con continuación y summary existentes, begin/prepare/finish/complete
compartidos por el wrapper actual. Seis casos nuevos en 280 líneas, sin ejecución.
Se conserva su consumo adyacente,
ExternalRuntime, NearbyCells, closure de carga única y todos los fences actuales.
Step completo suspendible implementado y revisado en fuente: step 254, pending
172 y diez nuevos casos en 339 líneas. Clock/spline/MM/policy se ejecutan sólo
en prepare; resume conserva phase/map/instance y no repite ese prefijo. Identidad
y admisión privada implementadas: acceso 84 y diez casos nuevos en 290/459 líneas;
los once casos de ActorStorage permanecen. Snapshot y transporte conservan Box y
witness; nueva admisión del mismo GUID recibe otra identidad. Nada está ejecutado.

Implementados y revisados en fuente acceso del manager (179 líneas), token
(414) y workset (428), con 15 casos nuevos: origen/epoch/incarnation/diff,
selección exacta, witness y slot se validan antes de la mutación Actor. La
operación movement mantiene queries/continuaciones owned; rechazos devuelven
continuación/respuesta y perder el request conserva busy. Otros 15 casos escritos
cubren los cuatro motores, terreno, RNG/clock, ABA, snapshot y accounting. La
terminación recuperable devuelve el token intacto ante error o Actor pendiente;
el wrapper anterior conserva firma y fases adyacentes. Ningún rechazo ejecuta
prefijo, Creature, tail o cierre de accounting. Mounts y exports integrados;
callback/raw actor y witness siguen privados. La barrera shutdown/quiescencia
está implementada. Sin ejecución, exclusión completa ni activación del productor.

### Contrato de activación del productor único (2026-09-30)

Dentro de cada mapa admitido, después de sesiones, respawns y DynamicObjects,
la operación representada conserva pases por fase: PlayerMelee una vez por mapa,
Lifecycle de seleccionados, Movement, Aggro, Spell y Melee. Completa publicación
antes del finish del mapa. No ejecutar todas las fases de A antes de B ni llamar
helpers globales legacy desde el token. Las colas de respawn pertenecen al paso
anterior a DynamicObjects; Lifecycle conserva muerte/cadáver/timer y sus submits.

La activación cambia deliberadamente scheduling: un diff/epoch, mapas completos
en secuencia y primarios del NearbyCells exacto. Fuera de selección no avanza el
update primario; víctimas, asistencia y split/share pueden recibir efectos sin
convertirse en primarios, con su identidad validada. Timers creados por Lifecycle
esperan el próximo ProcessRespawns; actores creados antes del plan pueden entrar
en ese plan. Se conservan fuentes y draws locales RNG, muestras temporales y
reglas/orden de destinatarios; desaparecen intercalaciones del loop independiente.
No acreditar equivalencia temporal o de RNG global con el bridge anterior.

Fundamento: World.cpp:2704/2748, Map.cpp:682-769 y VisitNearbyCellsOf, a5f8da2e.
Persisten los gaps de PlayerMelee posterior a DynamicObjects, orden Spell/Movement
frente a Unit.cpp:418-506 y AI/melee frente a Creature.cpp:696-850; su reparación
no se oculta en esta migración. LOS inicial de aggro/PlayerMelee, casts ausentes y
TempSummon tampoco se reparan aquí. Cobertura Actor/Pet, admisiones, todos los
lectores/escritores y retirada completa siguen siendo requisitos antes de activar.

El adaptador APP de Movement está implementado y revisado en fuente: 405 líneas
productivas, 360 de tests y once casos nuevos, sin ejecución. Requests propios
fuera de guards, resume validado y proyecciones finales estrechas conservan
MonsterMove/Stop, salud, trace y precedencia de salud durable, sin clonar Creature.
La admisión de fixture mueve el Actor real y sólo admite Inserted. Callback/raw
actor y witness permanecen privados. Timeout/drop/panic no completan token ni tick.

Entregada la disposición explícita de request no lanzado o reply owned ya observado:
consume continuación original y valida slot/origen/epoch/token; tras stale/ABA puede
liberar sólo el slot, sin Actor/tail/accounting. Rechazos devuelven todos los inputs.
El error APP conserva efectos parciales y retorna abandono explícito; query perdido
o panic sin continuación poseída sigue fail-stop. No convierte abandono en fase
completada ni autoriza activar el productor. Siete casos Map y nueve APP nuevos
escritos; cuerpos Map 77/APP 74, exports y los 16 casos revisados en fuente.
Admisión Fresh está implementada: API Map 84/manager 33 y 14 casos nuevos revisados
en fuente. Devuelve incoming íntegro ante errores/duplicados, no desplaza/promueve
Record y mueve el motor por el lifecycle compartido. AddToMap 778 es byte-idéntico
al PRE íntegro; su expect depende de que preflight cubra los errores actuales antes
de efectos y de que los hooks no reciban Map/entity store. Transferencia viva se
trata aparte. Partición física terminada: raíces inserción/removal/relocation/storage
125/351/223/107 y 14 children (máximo399). Parent compara 122 funciones originales:
121 cuerpos literales, coordinador reconstruido equivalente ignorando whitespace y
ObjectEntry byte-idéntico; PRE íntegro en /tmp/luna-map-physical-pre-20260930.
Respawn contratado: una definición de store y un slot por clave, SavedOnly, Catalog
o Actor. SavedOnly no aparece en INFO/time/queue; save temprano usa su API específica.
addInfo explícito de startup/pool sí crea Catalog. Actor conserva payload immutable,
Instant exacto y ordinal; los índices contienen claves, sin otro owner o scheduler.
addInfo sobre Actor conserva ejecutor/payload/Instant/ordinal y aplica el filtro Unix
actual a INFO. Actor reemplaza Catalog; Actor posterior se rechaza, anterior/igual
reemplaza y pasa al final. El namespace transitorio conserva GUID-low, no GUID completo.
Retirar INFO no cancela el payload Actor; cancelación owned es una operación distinta.
Catalog filtra ejecutores antes del corte temporal y conserva load-before-delete,
consume=true/false y unloaded actuales. Actor conserva su factory, guards y fallos.
Delta deliberado: desaparece la carrera Catalog/Actor por la misma clave; para Actor
gana su Instant/metadata, sin creación adelantada de catálogo ni pérdida por unloaded.
La implementación sustituye Vec/rows legadas por la misma definición y prepara una
transferencia move-only, sin activar producers. Dos instancias temporales permanecen
hasta migrar escritores bajo quiescencia y retirar el manager; no afirmar autoridad
productiva única antes. Fences DB/retry se conservan; no fabricar unknown-COMMIT.
Contraste Map.cpp:2191/3516 y Creature.cpp:419/2194/2651, a5f8da2e. Sin aceptación.

Revisión parent 2026-09-30: store/transport/pipeline Actor y 33 casos nuevos
leídos; 17/18 cuerpos Map idénticos, selector Catalog único delta, ocho Pending
y siete APP literales. Hallazgo pendiente del consumidor preparado: ground-snap
usa `LiveTerrainHeights::static_height_like_cpp` bajo el guard canónico y la
cadena `GridMapTerrain::with_tile → load_tile` puede hacer `std::fs::read`
(`terrain.rs:107`). Debe salir por query/continuación owned sin repetir factory,
RNG, saves o deletes, conservando deadline/ordinal y validando incarnation,
ocupación y disposición al volver. Revisar además la carga de grids durante
fresh admission; el helper preparado no acredita la frontera de I/O resuelta.

Corrección autorizada, ruta A del runtime concreto: `MapRuntime` usa
`Map<NoopTerrainGridLoader, NoopGridLifecycle>` (`map/runtime.rs:111`). Retirar
la nueva API genérica que permite callbacks externos bajo `&mut Map`; no
ampliar ni asumir seguros los loaders/hooks genéricos. Factory (incluida la
siembra de RNG por entropy) y ground-snap salen del guard y se ejecutan una vez.
La admisión concreta Noop vuelve bajo guard con el mismo motor, sin Clone.

La fase se admite desde el plan real aún `AwaitingSessions`: origen, epoch,
participant y incarnation antes de cualquier prefix. Un mapa por vez según
el plan, con sus ordinals originales: saves/corpse-removal/drain una vez,
rechazos iniciales GUID/spawn con sus Deletes actuales, request owned de
factory/snap, validación del reply, admisión y siguiente ordinal. No ejecutar
prefix de todos los mapas antes de resolver el primer mapa. Slot opaco conserva
identidad de operación; reservas del store contienen claves, no otro payload
ni rows duplicadas. Rechazar queue/INFO/save/cancel competidores sobre reservas,
con errores explícitos y ownership recuperable.

Mientras la operación está pendiente, no completar/abandonar el plan, transferir,
destruir ni descargar mapas. `destroy_map_inner` no libera instance IDs;
`unload_all` rechaza todo antes de mutar. Begin ya rechaza el tick en vuelo sin
avanzar timer. Reply valida origin/epoch/key/incarnation/reserva y ocupación.
Una ocupación nueva durante la consulta consume el intento como
`AdmissionRejected`, sin el Delete del rechazo inicial y conservando saved row;
no retry. Stale devuelve continuación y Actor para disposición explícita sin
mutar el reemplazo. Disposición valida slot y no acredita tail/accounting;
drop, panic o pérdida de query conservan Busy. No añadir reloj, task o lock.

APP conserva el fence existente a través de memoria y submission, libera
manager/entity guards durante factory/terreno y envía mailbox después de liberar
el guard, con el orden Save/Delete actual. Los 33 casos previos se conservan;
se escriben recording de try_lock fuera del guard, factory/RNG una vez,
ordinal/múltiples mapas, gates de timer/unload/destrucción/transferencia/reservas,
colisión inicial y sobrevenida, stale y disposición/pérdida. PRE íntegro de la
corrección separado del original. Sin ejecutar ni activar productores.

Aggro completo está contratado y asignado: conserva taunts → asistencia vencida →
threat/victim → adquisición → cola de asistencia y orden de candidates; usa el elapsed
de Movement, sin otro tick de clock/MM ni nuevos draws. APP conserva consultas de
catálogos/AI/distancia sobre IDs/facts y serialización; Map posee transiciones de
primarios y secundarios recíprocos. El LOS de asistencia sale del guard después de
distancia y antes de hostility, con continuación owned y efectos parciales. Los
secundarios pueden estar fuera de selección, pero su identidad se revalida; las
asistencias persistidas conservan GUIDs y resuelven el destinatario presente al vencer.
No se expone Actor/witness mutable ni se añade Map→data. Gaps de LOS inicial,
writers externos y retirada del productor siguen dentro de la puerta de activación.

Entrega Aggro revisada en fuente: motor común, backends, primary/threat/assistance,
admisión, continuaciones y driver APP, catálogos/wire y 28 casos nuevos leídos.
Los wrappers legados consumen el mismo motor. Map escribe refs recíprocas antes
del tail; pending mantiene efectos propios y gates de Player handle/residence y
Actor witness. PRE íntegro en `/tmp/rustycore-b6-aggro-pre`; diff de primary revisado.
Se retiró un doc huérfano; ambos forwards de aura están presentes en conversion.
Sin ejecución, exclusión de todos los writers ni activación. Russell pasa a Spell.

### Contrato completo de Spell (2026-09-30, entrega sin aceptación)

Parent revisa tick 818, validation 405, planning 316, metadata 441, admission 239
y publication 234. Contraste a5f8da2e: CombatAI.cpp:53-106/191-224,
UnitAI.cpp:61-79 y Spell.cpp:3839-3843/8056-8064/8363-8373. C++ exige scheduling
de CombatAI y el intento Turret con reset dentro de su rango estricto; los gaps
Rust de dificultad, targets, costos, cast completion y effects siguen explícitos.

Entrega de Russell: parent lee motor/protocolos/backends, admission/validation,
continuaciones/slot/disposición, catálogos/proyecciones APP, counters/wire,
driver canónico dormante y transporte legado con guards completos. Lee los
31 nuevos Map y ocho APP, incluidos sus helpers reales, casos de ABA, reply
retenido, publicación parcial, pérdida por panic y Drop sin settlement.
Comparación independiente contra HEAD d6c92488 de los cuatro archivos completos
`scenarios_world_entities_24..27` conserva literalmente 40 tests y sus helpers;
informe `/tmp/sol-spell-original-source-comparison-20260930.json`. Es prueba de
conservación de esas fuentes, no de todos los tests Spell, de ejecución o paridad.
El delta del root frente al PRE conserva los mounts de esta familia y añade el
target de los ocho casos APP; el retiro concurrente de PlayerItems5 pertenece
a su entrega separada. Sin compilación, pruebas ni activación canónica.

El motor compartido pertenece a módulos privados de Map, con backends legado y
Actor; APP conserva catálogos y wire. La entrega incluye wrappers actuales,
validación, planning, adaptadores de catálogos/publicación, consumidores y tests.
No clonar Actor ni copiar el algoritmo entre backends. Los callbacks reciben
IDs/facts inmutables específicos, consultados en el punto original; no exponer
Actor, storage ni witness, añadir Map→Data/Packet o precalcular un plan de
catálogos que cambie guards, consultas lazy o sus resultados ausentes.

Dentro de un mapa conservar preparación de todos los primarios antes de consumir
la cola ordered Cast/Schedule/TurretRejectedAttempt. Missing spell store devuelve
default antes del gate legacy. Alive/InCombat/victim Player preceden AI; CombatAI
conserva rechazo de dificultad/noninstant, initialize una vez, slots en orden,
Aggro/Combat/Die actuales y el clear del primer evento due antes de sus metadatos.
CASTING bloquea due después de initialize. Schedule conserva el gate de engagement
epoch y draw [minimum, minimum*2 saturado] en el consumo, nunca en preparación.
Random threat no representado conserva tombstone sin schedule. No nuevo tick
de elapsed, MotionMaster, fuente RNG o clock; usa elapsed ya avanzado por Movement.

La validación conserva alive/state/victim/epoch/incarnation, aura/control authority,
Player vivo, posiciones/reach actuales, range efectivo y latency de ambos movers.
Turret comprueba raw maximum con comparación estricta y resetea antes de cooldown,
CheckRange y LOS, incluso en los intentos rejected representados. Luego conserva
cooldown → min/max range → attributes → LOS → target hostility → hit metadata,
auras/vehicle/behind → hit profile/full log. NonTurret resetea un cast admitido
antes del rechazo de hit/log. Todo reject conserva sus contadores y efectos previos.

En el backend Actor preparado, LOS sale de guards mediante query y continuación
owned, sin Clone: revalidar
token/origen/epoch/map incarnation, Actor witness y Player handle/residence revision
antes de retomar; no repetir prefijo, reset, consultas ni draws. El slot queda
pendiente hasta completion/disposición explícita; drop/panic/timeout no completa
fase ni tail. Los secundarios fuera de selección no pasan a primarios. Mantener
el mismo orden lógico de acciones y efectos parciales durante prepare/resume.

Corrección de frontera, 2026-09-30: el slot no excluye las mutaciones directas
de Player ni los writers legacy; identidad/health no congela auras/stats/cast/
posición. La ruta compatibility productiva conserva canonical→legacy guards
durante facts/reset/LOS Noop/RNG/cooldown/GUID/wire append/Hit tombstone. Parent
lee `canonical_runtime/spell/legacy.rs`, wrapper validation y entrada canónica;
la búsqueda del identificador `run_canonical_spell` en todos los Rust de crates
sólo encuentra definición/re-export y tests, sin consumidor productivo actual.
El backend Actor owned permanece dormante hasta fijar y demostrar la exclusión
de sus writers; no acreditar equivalencia de interleavings ni ownership terminal
a partir de ese slot. No se ha compilado ni ejecutado la entrega.

Ambos perfiles hit requieren exactamente un roll 0..9999, incluso NO_ATTACK_MISS.
Missing hit/log y éxito Hit conservan sus tombstones RNG actuales; Miss no añade
uno. Cooldown canónico precede cast GUID y publicación. Map devuelve hechos owned
de cast, log (incluidos power types signed, orden y deduplicación), posición y
visibility; APP construye START/GO fuera del guard, con bytes, listas y recipient
rule existentes. La muestra temporal de GO permanece en append/publicación.
No implementar daño ni inventar costos/casts/targets para cerrar la migración.

Escribir equivalencia de backends, orden de acciones/queries/RNG/timers, rejects,
Turret boundary, missing metadata, fail-closed Player/Actor ABA, slot/disposición,
cooldown y START/GO/log/recipients. PRE íntegro en archivos antes de tocar fuente;
producción ≤600 y tests/fixtures bajo sus presupuestos naturales, sin compresión.
Sin Cargo/fmt/tests/metadata/QA durante barrida; sin activar productores. Esta
operación habilita la futura retirada, no prueba scheduler/owner único productivo.

### Creature Melee: motor síncrono compartido asignado, 2026-09-30

PRE íntegro `/tmp/luna-creature-melee-pre-20260930-094851/`: 32 fuentes Rust,
seis auxiliares, seis C++ completos y ledger nominal de 48 pruebas. Parent lee
tick, branches Player/Creature, shields, split/share, commits y replay CAS;
contrasta `Unit.cpp:2085–2246` con la referencia a5f8da2e. No ejecución.

Map asume el motor completo de readiness/admission, draw de daño, mitigación,
absorb school/mana, split/share y commits health/death/threat/timer. APP conserva
consultas específicas de catálogo, serialización, mailbox y bridge CAS legado.
Capturar resultados/values en sus sitios originales y devolver facts tipados;
sin Map→Data/Packet normal, Session/config bags ni nuevo Actor clonado.

La ruta compartida permanece síncrona con canonical MutexGuard → legacy
RwLockWriteGuard durante cada swing; conserva las ventanas entre swings y CAS
al final del batch. El slot actor existente no excluye los writers directos de
Player ni los legacy: liberar ambos guards y revalidar sólo identidad/health
permite interleavings nuevos de aura/stats/cast/posición. No crear reserva,
lock, cola diferida ni protocolo dormante para presentar ese delta como traslado.
LOS actual usa NoopTerrainGridLoader, sin ground/file I/O; mantiene dos consultas
reales, inicial y de commit, sus prefilters y endpoints, sin phase gate nuevo.

Conservar primary threat antes del gate inicial; draw Actor inclusivo, consumo
con bounds iguales, invalidación y max1; draw de tabla thread_rng inline en el
mismo hilo y sitio; fallback sin SpellStore; mana usa el snapshot de auras previo
a school y power posterior; split consume remainder; share mantiene float sin
clamp, slots ordenados y recheck live. Captura de wire health precede share y
sparring; rechazo de commit LOS conserva los commits secundarios. Clocks de
daño/muerte siguen separados. Retry100 frente a timer base/no-consumo, AddThreat0,
health→JustDied→0 y aura removal tardío actual se preservan; no reparar gaps C++.

Los 48 originales conservan nombres, inputs, helper assertions y operands. Sólo
casos que prueben el motor sin dependencias ascendentes migran a Domain; wire,
Session, seis commands, dos CAS y owner-noop conservan target APP integrado.
No atribuir pureza a los otros 39 por su nombre. Añadir casos directos de fases,
RNG y efectos parciales. PlayerMelee de `creature_melee_tick.rs:27–124` queda
fuera de esta familia. Mounts compartidos requieren turno con Spell/otros workers.
Huygens entrega motor y mounts en fuente, sin Cargo ni QA. Declara 48 nombres/
535 assertions conservados: nueve APP externos y 39 aún locales. Los 39 no están
externalizados ni aceptados; ninguna lectura de ese ledger sustituye contraste
completo de sus adaptaciones. Escribe nueve nuevos Map sin ejecutarlos. Parent
lee el engine completo de 450 líneas y el wrapper; no afirma cierre de todos los
helpers ni de equivalencia wire. PRE/delivery completos en
`/tmp/luna-creature-melee-pre-20260930-094851/`. El root Map compartido conserva
736 líneas y no cumple todavía su presupuesto. Huygens prepara ahora la fuente
canónica interna del mismo motor y la ubicación de los 39 casos restantes:
actor seleccionado/witness, guard síncrono durante swing, sin extraer al actor
del store que consultan los secundarios ni introducir un segundo algoritmo.
No activa productores ni cierra B6: quedan retirada del doble owner/bridge y
aceptación final completa.

### Player Melee contra Creature: motor compartido asignado, 2026-09-30

Russell termina la preparación y recibe implementación del motor único
`WorldCreature::apply_player_melee` en una familia privada de `wow-map`.
Contrato íntegro en `/tmp/rustycore-b6-player-melee-contract.txt`; PRE completo,
57 fuentes y 36 casos nominales conservados. La comparación de fuente del
método original frente a la raíz concurrente conserva sus 3 114 bytes; no es
aceptación. Los dos callers, global Player tick y Session combat tick, conservan
selección, geometría, timers/RNG del Player, guards, snapshots/sync y publicación.

Conservar dead admission, engagement anterior a readiness, `Some([])` sin
consumir readiness/RNG/timer de Creature, fallback `None`, presentación de daño
cero sin tap/threat, tap ordenado, health, full-damage threat, overdamage `as i32`,
lethal break/clear/stop, record sólo con `None` y values al final. Kill hooks,
loot/rewards y finalización completa JUST_DIED permanecen en el drain APP.
DTO específicos sin Packet/Data/Session, callbacks públicos de Actor ni clock
nuevo; los 36 originales y helpers se conservan y se escriben casos directos.

La raíz compartida `legacy_runtime/creature_melee_tick.rs` y los mounts/exports
Map quedan pendientes de integración exclusiva por Huygens. Russell entrega
motor/DTO 153 líneas y 16 casos directos/282; parent lee ambos archivos completos
y los tres snippets concretos de mount/export/delegado en
`/tmp/rustycore-b6-player-melee-integration/`. Los aliases locales World ya están
escritos y requieren esos exports. Se asigna aplicar esos snippets a Huygens,
preservando sus imports/motor Creature y los exports anteriores; aún no hay
confirmación de integración en ese momento. Parent confirma después los tres
mounts/exports/delegado en sus archivos reales. Contrasta independientemente los
36 cuerpos originales literales y las dos fuentes completas de callers sin
cambios contra PRE íntegro. Lee el diff completo del motor; sus tokens coinciden
tras excluir comments/whitespace y mapear sólo receiver y DTO paths. Informe en
`/tmp/sol-player-melee-original-source-comparison-20260930.json`. Sin Cargo,
fmt ni QA. Russell prepara la retirada efectiva del productor/copia legados,
incluida la exclusión de writers, transporte vivo y cancelación/shutdown; no la
activa ni sustituye el owner único por dormancy permanente.
Este corte no retira el manager legado ni acredita owner único o paridad completa.

### Transporte del owner existente: contrato de implementación

La preparación de Russell conserva 444 fuentes completas y el inventario de
constructores, lectores, writers, fases y disposición en
`/tmp/rustycore-b6-producer-retirement-pre/` y sus ledgers adyacentes. No acredita
que esas operaciones hayan migrado. El parent lee runtime_launch, object_work,
el bridge canónico, ObjectEntry, EntityWorld, Grid/MapInstance, inserción/índices
y el cierre del coordinador. Hay dos loops efectivos; el recibo actual de
quiescencia cierra shutdown permanentemente, sin contrato de reapertura.

Russell implementa el transporte move-only bajo quiescencia real de arranque
antes de toda admisión, o cierre permanente ya establecido. No introducir
handoff caliente, nueva reapertura, lock ni cola. El fence por sí solo y observar
Idle no prueban quiescencia de todos los writers. La futura composición debe
probar esa exclusión y sustituir los constructores por un único owner.

Orden fence→canonical→legacy; preflight del mapa completo antes de extraer:
map incarnation/Idle, GUID/kind/map/spawn, cardinalidad, duplicados y cobertura.
La contraparte debe ser Record Creature exacta; Actor existente, legacy huérfano
y Record sin motor se rechazan intactos, sin omitirlos del workset. El actor ya
in-world se transporta: no Fresh ni replay de AddToWorld/hooks. Take/restore
conserva su slot original y no vuelve a muestrear clocks ni reinicia runtime.

El inner ganador requiere la misma health timeline. Source gana con revisión
mayor, o igual y tuple health/max/death idéntico; en los otros casos comparables
gana el Creature canónico completo, movido dentro del mismo outer WorldCreature.
Nunca fusionar campos ni reconstruir MotionMaster/RNG/deadlines/create_data.
Ambas loot authorities deben compartir ya el almacenamiento. Cualquier autoridad
distinta se rechaza antes de efectos, incluso Pristine o Quarantined; no ejecutar
el reconciler de mirrors para fabricar consistencia ni crear/detachar tombstones.
Es un contrato de admisión del nuevo transporte, no una reparación del bridge
actual, que permanece intacto hasta retirar sus consumidores.

Validar ganador/índices antes del take; conservar losing inner y payloads hasta
reinserción exitosa y devolver/restaurar íntegramente ante rechazo. La rama
Source conserva el delta reciprocal-threat del bridge actual; Canonical no
publica incoming threat. Ningún ActorClone, tabla mutable adicional, callback
público raw ni unsafe. Casos escritos deben cubrir motor no predeterminado,
winner/revisión/tuple/ABA, divergencia loot sin efectos, colisiones/duplicados,
Busy/stale map y restauración. No conectar ni activar el transporte o retirar
productor hasta completar constructors/writers/lifecycle y composición real.

Russell entrega el transporte implementado, UNVALIDATED, en 15 archivos y 42
pruebas escritas: `/tmp/rustycore-b6-actor-transport-delivery.txt`, diff y PRE
adyacentes. Parent lee el informe y los seis snippets exclusivos; quedan por
revisar íntegramente admisión/commit/restore y aplicar los mounts. No hay caller
productivo, activación, retirada de copia ni cierre B6. Russell prepara el corte
de composición sobre factories/admissions/pipeline con el inventario existente,
sin volver a auditar las 444 fuentes ni activar el transporte.

**Transporte montado tras revisión parental de fuente:** parent lee completos
admisión183, commit101, Legacy128, manager32, APP51, los dos helpers EntityWorld
y toda la familia de 42 casos/fixtures. Integra los seis snippets root exactos.
Contraste independiente fuera de los dos helpers: EntityWorld conserva fuente
byte-idéntica frente al PRE, registrado en
`/tmp/sol-actor-transport-existing-source-comparison-20260930.json`.
El preflight borrowed completo precede cualquier take, no quedan errores públicos
recuperables después de empezar commit bajo los mismos borrows; los restores
devuelven los valores intactos. Permanece dormante, sin productor/caller nuevo.
Estas lecturas no son pruebas ejecutadas ni cierre de ownership/quiescencia.

**Corrección del contrato del nuevo transporte entregada, sin aceptación:** parent identifica
por fuente que aplicar reciprocal ADD/PURGE durante cada promoción permite que
un target posteriormente ganador Source pierda ese efecto al sustituir su inner.
El caso existente sesga ambos targets a Canonical y no cubre esa combinación.
Russell entrega la corrección y cuatro casos nuevos escritos, no ejecutados;
parent lee diff/casos completos y compara el archivo anterior de threat-tests:
es prefijo byte-idéntico del actual, incluidos los siete cuerpos originales.
Comparación fuente en
`/tmp/sol-actor-transport-reciprocal-source-comparison-20260930.json`.
La operación nueva de mapa completo retendrá sólo los deltas owned de Source,
promoverá todos los ganadores y aplicará después cada delta ADD→PURGE, conservando
el orden previsto de source y el mismo guard sin callbacks/clocks/await. Canonical
no aplica incoming deltas. Es un requisito de la transición atómica nueva, no
una modificación del bridge legado ni una reparación de gameplay; transport
sigue dormante. Russell conserva PRE y los 42 casos y escribe Source-peer/self/
mixed-winners adicionales sin ejecutarlos. Revisión y aceptación pendientes.

**Melee canónico montado:** parent lee Source178/engine403/entry51 completos,
los diffs de Player/Creature/secondary y los diez nuevos casos/fixtures. Integra
mod actor_melee y visibilidad pub(super) de selected_actor_witness, sin export
raw público. La entrada exige token actual→pending-slot Busy→workset/witness→
readiness→motor compartido, manteniendo el Actor en su Box original bajo el
mismo manager guard. Source es privado Legacy/CanonicalSelected; no extrae ni
clona Actor y conserva self-victim/share, partial commits y finish sin nuevo gate
alive. Huygens migra a continuación 38 APP originales y el delegate LOS Domain,
con 39 privados todavía intactos al inicio del turno. Ninguna activación ni
validación; PRE/POST e informe en
`/tmp/luna-canonical-creature-melee-pre-20260930-120825/`.

Zeno entrega contrato acotado de CreatureLoot en
`.codex-b6-creature-loot-contract.md`, 22 inputs PRE y mapa de callers. Parent
lee el contrato completo y asigna motor privado único sobre &mut WorldCreature,
operaciones semánticas concretas y resultados owned para Legacy/Canonical.
NormalRelease conserva force DynamicFlags antes del guard; DetachedCompletion
lo conserva dentro del guard unviewed. Instalación conserva orden alive/lifetime/
storage/retired/generation; ninguna await/SQL/publicación queda bajo guard.
Actor observado/rechazado, pending slot, map/incarnation o witness stale falla
cerrado; sólo ausencia real permite compatibilidad. El algoritmo dual-rebind de
claims permanece hasta integrar el handoff único; la ruta Actor queda dormante.
No hay selección por mera feature, mirror mutable, ActorClone ni nueva autoridad.

**CreatureLoot integrado en fuente:** parent lee motor159/Map42/manager231,
APP owner163/GO157/reconcile167, los trece diffs de funciones consumidoras y los
14 casos nuevos/55 assertions. Los 23 cuerpos trasladados son literales frente
al PRE íntegro por comparación independiente. Cuatro mounts/reexports integrados;
ningún caller productivo de Tick ni handoff se activa. APP conserva IO/RNG,
publicación y la reconciliación de ocho intentos.

Revisión posterior identifica que la ausencia física de un Actor admitido no
debe habilitar compatibilidad, y que el reader NoActor no puede entrar por el
mutador canónico general tras una nueva admisión. Zeno corrige ambas fronteras:
conjunto/witness congelado antes del shortcut y lectura específica CreatureRecord
que rechaza Actor bajo el mismo mutex. Parent lee el diff completo y cinco casos
nuevos/18 assertions; el archivo de siete manager-tests anterior es prefijo
byte-idéntico del actual. Informe/diff/PRE en `.codex-b6-creature-loot-correction*`,
comparación independiente en `/tmp/sol-creature-loot-correction-source-comparison-20260930.json`.
19 casos/73 assertions escritos en total, ninguno ejecutado. La ventana separada
NoActor→legacy write→snapshot sync y la identidad de las colas siguen abiertas.

La unidad reservada reutiliza begin/resume/complete_actor_operation del token
real para Loot async: reserva antes de facts/query, continúa únicamente la
identidad propia y no limpia slots en drop/rechazo. Parent revisa motor108 y diez
casos/50 assertions e integra el getter privado de epoch; el token sigue siendo
el único dueño del slot. Cuatro inputs anteriores (motor, actor access, token y
suite previa) son byte-idénticos, y el manager sólo añade mount y provenance en
`/tmp/sol-reserved-loot-source-comparison-20260930.json`. Las APIs Tick anteriores
conservan su rechazo de cualquier pending slot; la familia reserved es concreta,
preparada y dormante. Ningún Session GUID se convierte en token/witness nuevo.

El driver post-session reservado está implementado sin wiring: child90 mantiene
token/handle/source originales y try_finish_map devuelve el token en rechazo.
El wrapper anterior conserva su Option y descarte; no se activa otro productor.
Parent lee todos los cuerpos y once casos nuevos/48 macros por caso más cuatro
del helper compartido. Begin/prepare/complete y el archivo de seis tests previos
son literales en `/tmp/sol-object-loot-driver-source-comparison-20260930.json`.
Las pruebas controlan un await y disposal explícito, pero no se han ejecutado ni
integran generación real. Request settlement/join sigue siendo obligación del
caller futuro. Finalización recuperable del tick y APP tail están implementadas
y revisadas: se devuelve la continuación/work/summary original en preflight;
Ok(None) queda finalizado, sin replay, y wrappers anteriores conservan descarte.
Parent lee once casos nuevos/62 assertions por caso más siete compartidas y
confirma suffix completo de efectos, trece funciones manager, cuatro APP y siete
inputs originales literales en `/tmp/sol-final-object-tick-source-comparison-20260930.json`.
La composición tipada del consumidor está implementada y revisada: diff672/seis
paths y ocho casos nuevos/64 macros escritos. Conserva fallo/work/token y la misma
TickAdmission; guards se sueltan antes del siguiente await y el interval guard
impide clock/admission/pass nuevos. Ok(None) continúa PostTail/FullyFinished,
reparando la regresión None→break de este WIP, ausente en HEAD. Parent confirma
ocho inputs anteriores, diez funciones completas, closure GameObject y suffix
entero de publicación/PostTail/shutdown literales en
`/tmp/sol-object-consumer-source-comparison-20260930.json`. Productor vacío real
cubierto por prueba escrita; fallo retenido cubre work/ledger reales, no una
inserción artificial en el spawn. Begin recuperable está implementado y revisado:
diff624 completo y diez casos nuevos/105 assertions escritos; conserva plan antes
del prefix y error/plan/summary después, sin replay. Parent compara trece inputs
completos, catorce funciones Manager, seis APP y siete casos del consumidor
literales, además del suffix BEGIN de éxito y prefix completos en
`/tmp/sol-object-begin-source-comparison-20260930.json`.
AfterPrefix sólo tiene prueba de empaquetado de frontera, sin afirmar un fallo
alcanzable en el flujo normal de préstamo exclusivo. Retry con settlement
real y transferencia tras abort permanecen abiertos. No pruebas ejecutadas,
auto-retry ni quiescencia atribuida a abort.

El siguiente retorno por stop cooperativo está contratado (18:05 UTC): el mismo
task devuelve el failure y la misma TickAdmission por JoinHandle tipado cuando
el stop existente está solicitado. Supervisión conserva el resultado original,
incluso tras consumirlo en select!, y clasifica RetainedObjects como incompleto.
Sin stop, el guard conserva su fail-stop sin nuevo clock/admisión/retry. No cambia
orden de quiescencia/drain/stop ni permite receipts desde retorno/abort/drop;
mantener payload durante supervisión no prueba disposición ni settlement.

Retorno cooperativo integrado en fuente18:38: parent lee diff515 y los siete
casos nuevos/55 macros, compara cuatro archivos completos fuera de los cambios
explícitos y quince inputs literales en
`/tmp/sol-cooperative-stop-source-comparison-20260930.json`, y monta producer_exit.
El success suffix/PostTail y los gates/stop.store de supervisión permanecen
literales. El resultado retenido vive en el mismo scope de shutdown; JoinError,
timeout o RetainedObjects no prueban settlement y dan apagado incompleto.
Contraste posterior18:58 corrige la hipótesis anterior: quiescencia fallida marca
ERROR y continúa el shutdown hasta stop.store; no retorna antes. El resultado
retenido sobrevive los awaits siguientes, pero se descarta al devolver ExitCode.
Esta entrega no resuelve disposición ni settlement y sus tests no se ejecutaron.
No se implementa la propuesta de outcome privado→startup que acabe igualmente
descartándolo. Falta accounting real de Objects con el mismo manager/ticket.
Abandono recuperable del plan queda contratado en
`.codex-b6-recoverable-abandon-contract.md`: mismo can_resume gate/efecto Idle,
rechazo devuelve plan original y BeforePrefix lo retiene. AfterPrefix devuelve
failure original sin tocar manager. Wrapper compatible conserva sus resultados;
éxito sólo MapIdle, sin receipts, nuevos phases o settlement de TickAdmission.
Abandono recuperable integrado19:47 en state_2 por snippet parent: childMap23 y
APP18, seis casos nuevos/127 macros escritos. Parent revisa cuerpos/casos y
compara diez inputs completos, dos roots APP y wrapper Map íntegro fuera del
mount/delegate en `/tmp/sol-recoverable-abandon-source-comparison-20260930.json`.
Nuevo consumidor BeforeObjects asignado: ante rechazo o manager poisoned retorna
directamente plan completo/participants/TickAdmission/permisos originales en los
dos antiguos puntos break. Causas poison específicas, sin recuperar manager;
éxitos conservan retained_tick o AbandonedAfterAccounting en sus puntos originales.
No había receipt falso en los rechazos anteriores. No nuevo held cell, espera por
stop, retry, clock ni disposición; stop/join clasificará el retorno incompleto.

El caller real investigado es queue_pending_creature_kill→process_pending_kills→
ensure_creature_kill_loot. Su cola guarda sólo GUID; el Object token real nace
después del drain Session. Capturar un handle allí no recupera la identidad de
la muerte original ni permite reservar retroactivamente esa fase. Contrato/PRE
en `/tmp/rustycore-b6-kill-loot-caller-pre-20260930-luna/contract.md`. El driver
preparado protege generation-origin solamente; cerrar el handoff sigue exigiendo
identidad capturada por el productor en la transición real, sin fabricarla desde
un lookup posterior, y retiro de las ventanas NoActor/write/sync y de la cola ABA.
El análisis posterior confirma que melee no crea un slot original de víctima y
que Spell valida otro primario/víctima Player; no se reutilizan como prueba del
handoff. Se elige B para diseñar el primer slot en killed real bajo el guard
original, root del atacante seleccionado y capture de targets realmente dañados
sin extender el workset de update. Parent lee completos los cinco archivos de
engine/commit/split/share, gates y tipos actuales; el collector específico que
presta el token original y reserva una vez está implementado, revisado e integrado
en Map: entry100/collector165, cinco archivos del motor≤417 y 14 nuevos/113
assertions escritos. Parent lee cuerpos nuevos/diffs completos/casos/fixtures;
comparación independiente de las catorce funciones originales completas tras
retirar sólo threading/capture y variables del mismo lookup, cinco cuerpos
incluida la entrada anterior literales y cuatro inputs previos íntegros en
`/tmp/sol-original-melee-kill-capture-source-comparison-20260930.json`.
Root reexport integrado; source capturada es postcommit, no posthooks.
Sin Entity clone, segundo slot ni modificación del flujo actual por captura
fallida; install/finalize/settle/dispose y consumer completo esperan su contrato.
Parent vuelve a leer Unit::Kill10457–10763 completo en a5f8da2e: Loot precede
KillRewarder/procs Kill/Death; AI/ScriptMgr son posteriores. "After hooks" no
define un punto único antes de generación sin invertir esa secuencia. La captura
actual es postcommit/before-hook; las trazas opt-in existentes no son un executor
y el driver Reserved reserva TARGET, distinto del slot ROOT original. La siguiente
continuación requiere el mismo ROOT y witness TARGET capturado, sin nuevo slot
ni lookup receptor convertido en identidad; no hay completion artificial.
Pending original ROOT/TARGET está contratado para validar la primera ocurrencia
y devolverla con el mismo token/outcome/batch/cursor ante rechazo. Esta unidad
no ejecuta hooks, avanza cursor, genera loot ni dispone el slot; Source capturada
permanece before-hook/postcommit, y retorno/drop no prueba settlement.
No activación ni cierre de Kill. No se implementa un receiver-only nuevo para
afirmar que el productor existe. Finding/PRE en
`.codex/canonical-melee-kill-producer-presnapshot-20260930/ORIGINAL_SLOT_FINDING.md`.

**Factory loaded-grid preparada:** parent lee los cinco archivos nuevos, sus
15 casos escritos y todo el contrato; confirma que grid/spawn/respawn_catalog/map
originales son archivos byte-idénticos y aplica los dos mounts exclusivos.
Informe/PRE en `/tmp/rustycore-b6-prepared-loaded-grid-*`; comparación independiente
en `/tmp/sol-prepared-grid-original-source-comparison-20260930.json`.
El motor preparado se construye antes de Fresh y el mirror legacy después de
Record Add: no se afirma equivalencia temporal ni se cambia el caller productivo.
La admisión compartida de facets/primary entre Pool/Conditions/Catalog y la ruta
preparada está implementada y parent integra Map/lib: motor107 y diez casos
nuevos revisados completos, sin ejecución. Record conserva su snapshot después
de pre-adds y antes de Add; Actor devuelve sólo receipts e incoming íntegro.
Timers, planners, counts y firmas legacy permanecen en su punto original. Parent
confirma los cuatro archivos consumidores completos fuera de sus bloques
sustituidos y los tres archivos/15 casos prepared previos literales en
`/tmp/sol-shared-loaded-grid-source-comparison-20260930.json`. Group/Conditions
owned está revisado e integrado: dos roots de exports, core266, routing127 y doce
casos nuevos leídos completos. Firmas, gates, match/counters/push Record y motor
shared conservan fuente literal con sólo los wrappers/DTO revisados; dos funciones
oracle y trece inputs anteriores permanecen literales en
`/tmp/sol-group-owned-loader-source-comparison-20260930.json`. Los duplicados Actor
devuelven incoming completo y bloquean Add según el nuevo contrato dormante;
no equivalencia con refresh Record ni activación productiva. El siguiente corte
Pool está implementado, revisado e integrado con dos exports parent: root571,
motor282, routing216 y despawn139, ocho casos nuevos escritos. Parent lee diff1887
y confirma cuatro motores recursivos tras aliases/receipt args, cinco funciones
despawn, seis oracles, trece funciones restantes y 23 inputs completos literales
en `/tmp/sol-pool-owned-source-comparison-20260930.json`. NoLoader conserva None
real; Unavailable y rechazo retienen la distinción y Records completos. Un solo
planner, mismos callbacks/cursors/orden y counter/push Record en su sitio; no
rollback de efectos parciales y Actor conserva incoming/motor. Catalog está
implementado, revisado e integrado con dos exports parent: root338, routing270
y doce casos nuevos escritos. Parent lee diff1154 completo y compara 36 inputs
completos, dos wrappers y nueve funciones routing literales; oracle original
íntegro, cola completa fuera de los tres bloques receipts revisados y match Pool
compartido íntegro salvo metadatos plan en
`/tmp/sol-catalog-owned-source-comparison-20260930.json`.
Usa el mismo core/sink: pooled acciones antes de timer removal, no pooled
loader antes de removal antes de preadds/admisión. Failures/consume/continue/break
se conservan; caller APP sigue Record. No promoción instalada ni pruebas
ejecutadas, y la diferencia temporal de factory sigue abierta.

El par Creature/Record clonado del factory productivo está retirado en fuente
18:38: un único core privado131 devuelve la Creature creada, el proyector Record
real la mueve en el punto actual y el resolver compatible conserva DTO/clone
para sus readers existentes. Parent lee diff386 completo y seis casos nuevos;
algoritmo/proyección/caller/provenance y21 inputs completos contrastados en
`/tmp/sol-created-creature-source-comparison-20260930.json`; dos mounts integrados.
Son tres paths APP; settlement conserva Option/logs/efectos parciales actuales.
Prepared recibe automáticamente ese mismo builder real, sin otra factory ni
activación. La raíz717 sigue fuera del presupuesto. El futuro rechazo owned
completo tras CastGUID requiere contrato separado; no se atribuye retiro de un
algoritmo World ni resolución de timing ni aceptación desde estos tests escritos.
Continuación19:47: fábrica/provenance común ya integrada, root942→791 y motor222;
dos preparadores typed usan el mismo algoritmo. Error move-only conserva Records
enteros y efectos parciales CastGUID/provenance previos, sin reallocation/rollback.
Wrappers Option/logs siguen compatibles. Diez nuevos casos escritos; tres cuerpos
completos contrastados con sustituciones de retorno explícitas, dos visualizers y
dieciocho inputs íntegros conservados en
`/tmp/sol-owned-provenance-source-comparison-20260930.json`.
El contraste de timing no demuestra equivalencia del preparador temprano: el
snapshot viejo se toma antes de Add y el wrapper viejo se crea después de éxito,
en el punto de mirror real; las otras familias conservan sus puntos de lote.
Se diseña finalización de birth sobre el único Creature canónico post-Add, con
create_data previo y conversión privada de tabla sin remove/Add/índices/loot
repetidos. Mantener RNG/waypoint en el punto original y no repetir Motion.add.
Contrato/CPP completos en
`/tmp/rustycore-b6-loaded-grid-construction-timing-proposal.txt`; todavía diseño,
no integración productiva, retirada de mirror/productor ni aceptación.

**Kill representado integrado:** parent lee motor121/selected32, quince casos
nuevos/81 assertions y sus helpers, y ambos diffs APP completos. Los cinco archivos
con ocho originales/54 assertions permanecen byte-idénticos; los dos consumidores
conservan el archivo entero fuera de sus respectivas closures sustituidas.
Dos mounts integrados; observabilidad log→stop serialization→values y death→flags
se mantienen. Session conserva legacy, sin fabricar identidad desde sus colas.
La externalización preserva los cinco APP/27 assertions, añade instrumentación
disponible con feature y activa exclusivamente el modo fixture existente, antes
de cualquier lectura de trace. Los seis mounts están integrados y parent revisa
cuerpos, helpers y wiring; comparación fuente independiente en
`/tmp/sol-kill-app-external-source-comparison-20260930.json`. La ampliación de tres
inputs históricos Pet únicamente en el modo handleless opt-in está revisada e
integrada: tres ramas feature y seis gates en dos roots, con valores existentes.
Parent lee tres casos nuevos/25 assertions y compara archivos completos con PRE
en `/tmp/sol-kill-pet-input-source-comparison-20260930.json`; cfg(test), feature
normal y Some(stale) conservan sus contratos. Los cinco originales y helpers/tres
casos de wiring previos siguen literales. No aceptación ejecutada ni AI/scripts
nuevos; Pet613 conserva deuda física.

Einstein conserva PRE completo de 25 Rust/siete funciones C++ y 14 originales/
107 macros directas. Parent lee Creature::Update completo, Entity runtime plan,
el prefix canónico Actor y lifecycle legacy. La preparación suponía que el
lookup legacy de spawn-id admitía peers muertos; la fuente real
`map_manager/respawn.rs:234–249` exige is_alive y desmiente ese supuesto delta.
Se corrige la propuesta; no adaptar comportamiento a ese supuesto.

**Prefix común integrado después de entrega:** Einstein entrega un único
algoritmo memory-only de death-save/corpse/row/queue/info/removal/ready y dos
backends privados Canonical/Legacy, con los consumidores reales escritos.
Parent lee engine113/backend183/model43/facade21, Actor64/World251 y todos los
nueve casos nuevos/94 assertions y helpers. Integra los tres mounts exclusivos
de respawn/map/legacy manager. World retira 108 líneas de algoritmo duplicado
(359→251), canonical adapter 75 (139→64); la nueva familia queda <=600 por archivo.
El algoritmo comparte lógica, pero las dos instancias de storage siguen vivas.

Contraste parental independiente con PRE íntegro: initial guard/settlement/delete
canonical conservan funciones literales y el tail World desde for-ready hasta
publication/refresh coincide byte por byte. Informe
`/tmp/sol-creature-common-prefix-source-comparison-20260930.json`; PRE/delivery en
`.codex/creature-common-prefix-presnapshot-20260930/`. Canonical captura attached,
Legacy captura el valor original después de remove y lo retiene hasta terminar
la iteración; save→clear, cleanup→capture, upsert sin Delete publicado y los
efectos parciales de removal fallido permanecen. Ready reserva sólo Canonical y
drena Legacy. Mismos clocks suministrados y prefijo→factory del mismo mapa antes
de avanzar; no nuevo postDynamic plan. Los 14 originales/107 macros y helper5
quedan intactos según entrega; no se afirma que hayan pasado ni aceptación.
Einstein prepara el siguiente corte de complete-death/Kill y sus escritores,
coordinado con Loot/Melee, sin nuevas auditorías globales o activación.

El lifecycle productivo existente es death-save/corpse/queue/factory/settlement,
ya representado en el prefix Actor preDynamic. Production selecciona
ExternalRuntime y no ejecuta Entity::runtime_update_plan postDynamic; habilitar
sus Boundary/regen/AI/actions no sería un traslado de comportamiento existente.
No se autoriza ese nuevo motor ni duplicar prefix o clock. Los gaps Unit/AI
permanecen explícitos. Einstein externaliza los 13 originales APP World,
conserva el decimocuarto en su target productivo WorldServer y prepara reutilizar
el mismo prefix real en vez de agregar otro algoritmo. Escribe cobertura de
la separación real/token/clock sin ejecutar ni activar producers.
La entrega de Einstein conserva 13 APP externos/96 macros y el original
WorldServer/11, más cinco macros del helper y siete casos nuevos/48 sin ejecutar.
Parent lee la entrega/propuesta común completas, creature_tick255, facade38 y
los cinco nuevos Map/172; aplica los dos snippets concretos de montaje.
No se retira masa productiva de World en esa entrega. El rail de fase es opt-in
por operación: normal false, resolver original primero, default PhaseShift sólo
si None y sin handle en los dos sitios originales, sin instalar Player.

Después del contraste de ambos prefixes reales, Einstein recibe implementación
de un único algoritmo privado en Map con backend cerrado Canonical/Legacy.
Conserva GUID order propio, death-save antes de corpse selection, save→clear flag,
cleanup antes del capture, upsert si anterior sin SQL Delete, queue/info/removal y
ready clocks suministrados sin recaptura. Legacy retira físicamente antes de
capturar el mismo valor movido; Canonical captura prestado y conserva terminal
removal/counters/efectos parciales después de queue/info. Ready mantiene DRAIN
Legacy y RESERVE Canonical. El adapter APP conserva por cada mapa prefix→factory/
commit antes del siguiente mapa y buffers/mutex de submission/publicación donde
están. No Actor clone, nueva autoridad/store/clock ni activación postDynamic.
Mounts/exports compartidos se integrarán desde snippets por el parent.
Huygens entrega la preparación y recibe la implementación del mismo motor Melee
con una fuente privada Legacy/CanonicalSelected. El corte usa accesos breves
por fase al Actor bajo el mismo guard del manager, sin extraerlo ni clonarlo.
La entrada canónica valida token/mapa, selección/incarnation y witness antes
de queries/RNG/writes; rechaza cualquier actor_operation pendiente y no abre
otro slot. Legacy conserva su prueba entre autoridades, alineación, ventanas
entre swings y CAS final; la ruta canónica no reproduce ese bridge.
Se conservan self-victim/self-share, efectos secundarios parciales y los puntos
actuales de catálogos/reset, sin añadir un gate alive tardío. Mount y visibilidad
del validator se integrarán desde snippets, sin editar raíces compartidas.
Los 39 originales pendientes tienen destinos preparados (38 APP, uno Domain);
se trasladarán después del motor común, conservando los nueve externos previos.
Esta implementación no activa productores ni acredita quiescencia o cierre B6.

La campaña final del programa debe cubrir: una transición y un diff por tick;
orden World/Map/Creature/tail; selección NearbyCells efectiva; sustitución de
incarnations antes de efectos; fallo de inserción, actor existente, evento
unspawn, respawn, remove y unload; fence de mutation-to-submit; cancelación y
shutdown con closure en vuelo; todos los consumidores del manager retirado.
Retirar la segunda copia y los puentes sólo después de migrar su motor y estado
real: MotionMaster, splines, RNG, deadlines, AI/combat y autoridad de loot/aura.
Clonar WorldCreature no conserva esa vida (su Clone reconstruye MotionMaster).

Pruebas se escriben durante la barrida y se ejecutan en la campaña final única.
La aceptación local no sustituye la evidencia de runtime requerida ni concede
autorización de reinicio, publicación o cambios de base de datos.
