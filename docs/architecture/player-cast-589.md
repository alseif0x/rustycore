# Represented Player cast-request lifecycle — #589

## Traspaso solicitado — 2026-09-08

El usuario ha autorizado analizar y documentar lo pendiente para otra IA;
**no ha reanudado el desarrollo**. Esta revisión consultó #589 en GitHub
(OPEN), Git, el código de los adaptadores y las pruebas, y los tres logs
focalizados citados abajo. No ejecutó compilaciones, tests ni QA nuevos.
La conclusión es **implementación parcial con aceptación incompleta**:
no está demostrado que sólo falten pruebas, ni hay base para un porcentaje.

### Dónde está el trabajo

- Checkout de implementación: `/home/server/rustycore-cast-589`.
- Rama: `589-archcore-complete-represented-player-cast-request-lifecycle`.
- HEAD: `cc8a8e97c562aa07c8c85185b8764dc40a8f9c0e`; los cambios de #589
  siguen sin commit, incluidos archivos nuevos sin seguimiento. Clonar la rama
  remota o copiar sólo `git diff` **no recupera estos archivos nuevos**.
- `/home/server/rustycore` sigue en #587, con dos documentos modificados y
  `docs/architecture/lfg-343-audit.md` no relacionado: conservarlos.
  Este checkpoint vive en el checkout de #589; no confundir ambos árboles.
- La rama contiene antecedentes locales de #587/#588. Antes de publicar,
  verificar su integración real para que el PR de #589 no arrastre entregas
  pendientes inadvertidamente. Esta revisión no publica ni integra nada.
- Las instrucciones compartidas todavía dicen que no se eligió familia tras
  #587; ese dato está desactualizado frente a la aprobación explícita de #589
  y su issue abierta. No reiniciar la selección ni reabrir #585.

### Qué falta realmente para aceptar #589

| Frente | Estado observado | Condición de cierre |
| --- | --- | --- |
| Admisión y preparación | Aplicación privada y adaptadores presentes; pruebas focalizadas verdes en sus ejecuciones registradas | Verificar operación completa instantánea y temporizada, cola dentro/fuera de 400 ms, reemplazo y revalidación con composición real |
| Identidad y vida del cast | Asignación Map, revisión de residencia y pruebas de exclusión presentes | Acreditar secuencia compartida con el consumidor criatura real, continuidad al reingresar y cancelación pendiente sin consumir identidad; no sustituir integración por un test directo del generador |
| Publicación y efectos | Prepare/Start/Go e interrupciones conectados; observadores usan el rail de #588 | Capturas nuevas pareadas de emisor y observador que prueben bytes, identidad, visual, targets, conexión, orden y ejecución única |
| Consumidores compartidos | Cambios en toy, binder, spell-click y adaptadores de interrupción; executor compartido conservado | Regresión explícita de first-login, item acquisition, self-resurrection, loot, transferencia, stance, canal y stuck; preservar owners, registro de opcodes y driver |
| Payload y condiciones | Serializadores tipados presentes; productor Player rellena RemainingPower y deja otras secciones por defecto | Resolver cualquier sección, selección de targets o condición visual requerida por los escenarios aprobados; serializar un campo no demuestra que producción lo calcule |
| Bot integrado | Modo cast/cancel/wait y recogida de observador presentes | Formato y pruebas del estado actual, incluidas las últimas restricciones de IDs y eventos inesperados; validar el evaluador antes de usar su PASS como evidencia |
| Arquitectura | Métodos movidos y nuevo comando; política de ownership sin cambios correspondientes | Revisar delta semántico, actualizar sólo referencias justificadas y ejecutar checks de ownership, persistencia/snapshots aplicables y política física |
| Regresión final | Sólo campañas focalizadas acreditadas | Suites afectadas de packet/entities/map/world, consumidores y composición world-server; QA aplicable al conjunto final |
| Publicación | No hay commit de #589 ni aceptación terminal | Registrar candidato y evidencia exactos; commit/push/PR/merge únicamente con autorización correspondiente |

Hay una corrección local concreta pendiente de política física:
`session_tests.rs` mide **95.705 líneas**, frente al techo **95.703** en
`tools/architecture/physical-file-policy.json`. Resolver por una separación
coherente de responsabilidades o delta expresamente justificado; no regenerar
la política a ciegas. `handlers/spell.rs` mide **6.272 líneas** en esta revisión:
su cierre necesita revisar la organización de la responsabilidad afectada,
no declarar modularidad porque se haya reducido el archivo o pase un techo legacy.
Estos conteos son inspección; no se ha ejecutado el validador de arquitectura.

### Incertidumbres que pueden exigir más implementación

El resolver visual no acredita evaluación general de UnitCondition. El productor
Player de `session/player_cast/publication.rs` construye RemainingPower y usa
valores por defecto para las demás secciones. El checkpoint conserva pendientes
la autoridad de estadísticas del combat log avanzado y comportamiento completo
de runas/proyectiles. Hay que decidir su necesidad a partir de los casos de #589
y datos efectivos, no excluirlos automáticamente para hacer pasar una fixture.
El motor completo de efectos, SpellHistory, pets/vehicles y todas las reglas de
targets no pertenecen íntegramente a #589, pero cualquier dependencia necesaria
para sus casos de aceptación sí debe resolverse. El resto continúa bajo #30/#584.

La QA de observador en modo `observe_only` recoge evidencia pero no acredita
aceptación por sí sola: requiere correlación con el emisor. La fixture sintética
30798→6197 heredada de #587 no demuestra gameplay stock ni cubre la matriz
instantáneo/temporizado/cola/cancelación/fallo tardío completa.

### Evidencia que se puede conservar y límites

Los logs existentes confirman **19/19** del filtro `player_cast`, **81/81** de
`handlers::spell::tests` tras reparar fixtures y **11/11** del bot en su ejecución
anterior. No sumar esos filtros como cobertura total ni atribuirlos a HEAD limpio:
se ejecutaron sobre cambios sin commit y no existe aquí un manifiesto que fije
el contenido exacto de cada archivo probado. Las últimas modificaciones del bot
son posteriores a sus once pruebas. No se volvió a ejecutar ninguna en esta revisión.

Los cuatro fallos corregidos eran fixtures de aura/mover; ese arreglo no acredita
por sí mismo paridad de casts. Tampoco la aceptación previa de #587/#588 valida
los consumidores modificados por #589. No hay capturas live nuevas de #589.

### Cómo continuar sin perder contexto

La siguiente IA debe leer este checkpoint y #589, inspeccionar ambos worktrees
y preservar todos los cambios tracked/untracked. Primero cerrar las incertidumbres
y ajustes de implementación dentro de la macro; después una campaña final
secuencial con un job Cargo, incluida la composición de producción. Usar el
checker construido desde el checkout correcto, no un binario antiguo que tenga
otra raíz incorporada. Leer las guías del bot antes de preparar QA real; servicios
y bases de datos requieren autoridad concreta. No hay permiso nuevo para ello.

Las referencias son el Classic 3.4.3 versionado citado abajo y AzerothCore como
fuente complementaria de gameplay; no copiar formatos wire/SQL 3.3.5. No es
necesario reiniciar una auditoría global ni crear micro-issues. El orden global
sigue siendo núcleo requerido de #584 → #583 → #153 → cierre de #133.


Status, 2026-09-08: approved and implemented **in progress**, based locally on
`cc8a8e97c562aa07c8c85185b8764dc40a8f9c0e`. The working changes are uncommitted.
The user requested a stop after correcting and verifying the four handler-suite
failures. That correction is finished; work is paused. Focused evidence is below.
Architecture checks, complete regression acceptance and live QA remain pending.
Earlier #587/#588 acceptance does not validate these changed inputs.

The approved scope and acceptance remain in
[#589](https://github.com/alseif0x/rustycore/issues/589) and the
[architecture plan](modularity-and-ecs-plan.md#next-core-candidate--2026-09-08).
This checkpoint records implementation and evidence, without creating a new plan.

## Ownership and consumers

`wow-world::player_cast` coordinates normal request admission and preparation.
Its private Session adapters resolve canonical state, catalogs, power and packet
publication. Pending state stays in Player gameplay state; active execution and
cooldowns stay in Unit spell execution. The existing Session driver completes
active casts before admitting pending casts. There is no new timer, task or
mutable state mirror in production.

Normal immediate and queued requests use the same preparation path, retain the
client request ID until SpellPrepare maps it to a map-allocated server Cast GUID,
and stamp active preparation with the canonical residence revision. Instance-local
Map allocation is shared with represented creature casts. Stale or detached Player
handles cannot allocate; a prepared cast cannot execute after residence reentry.

Toy, binder and spell-click consumers adapt to fallible canonical GUID allocation.
Existing server-triggered shared-executor consumers retain their explicit metadata
contracts, including first-login, item acquisition and self-resurrection. Normal
client defaults are not imposed on those callers. Loot, movement/stance, channel
and transfer interruption reach the same active state owner.

Start/Go and interruption frames use the existing bounded durable directory rail
for observers, after recipient visibility and map selection. Session delivery
checks connection generation through the directory, current map and shared client
visibility storage. No packet delivery occurs while the canonical map guard is held.

## Intentional behavior repairs

These are #589 repairs, not claims of behavior-preserving movement:

- Immediate casts now prepare and start before Go; queued requests revalidate
  knowledge, override, checks and power before preparation completes.
- Normal server Cast IDs, empty OriginalCastID, effective server visual, Start/Go
  flags and power sections replace the former client-derived metadata.
- Preparation starts the represented GCD once; launch does not restart it.
  Active cancellation clears that preparation GCD. A late power failure retains it.
- Cancellation uses SpellFailure then SpellFailedOther before Interrupted result.
  Late launch rejection uses its specific CastFailed before those interruption
  frames. Mismatched CancelCast still cancels pending requests when a cast is active.
- Power consumption rechecks resources under the same canonical guard as the
  debit, and cannot report success after losing the owner before deduction.

Exact Classic source at `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`:
`Entities/Player/Player.cpp` RequestSpellCast/CancelPendingCastRequest/
ExecutePendingSpellCastRequest/CanExecutePendingSpellCastRequest;
`Handlers/SpellHandler.cpp` HandleCastSpellOpcode/HandleCancelCastOpcode;
`Spells/Spell.cpp` constructor, prepare, cancel, _cast cleanupSpell,
SendSpellStart, SendSpellGo and SendInterrupted;
`Server/Packets/SpellPackets.cpp` SpellCastData and interruption writers;
`Maps/Map.h` GenerateLowGuid and ObjectGuid construction.
These source anchors motivate the contract; fresh paired captures remain required
and source code itself is not proof of correctness.

## Focused evidence and requested pause

All commands ran locally on aarch64 at the uncommitted working changes above
`cc8a8e97`; none establishes that committed base as containing #589. Cargo used
one job and existing target caches, sequentially; the standalone bot has its own
manifest. Workspace builds used `PROTOC=/home/ubuntu/.local/protoc/bin/protoc`.

- `cargo test --locked --offline -p wow-world --lib player_cast`: **19 passed**,
  zero failures. Log: `/tmp/rustycore-589-player-cast-tests.log`.
- `cargo test --manifest-path tools/wow-test-bot/Cargo.toml --locked --offline
  --bin wow-test-bot cast_lifecycle`: **11 passed**, zero failures at that run.
  Log: `/tmp/rustycore-589-bot-cast-tests.log`. Subsequent evaluator hardening
  rejects all unexpected plan-bound cast events and reused client request IDs;
  its additional plan test has **not been run**. Do not label the latest bot
  changes as validated by the earlier eleven-test result.
- `cargo test --locked --offline -p wow-world --lib handlers::spell::tests`:
  first **77 passed, four failed**. Canonical fixture adoption exposed three
  mount aura setups still writing the obsolete Session-only test field, and a
  fixture missing the Player's self-mover initialization. The aura setups now
  use `insert_player_visible_aura_like_cpp`; the common canonical fixture sets
  the moved-unit GUID explicitly. Production mount/totem behavior was not
  changed to accommodate these fixtures. The corrected suite **81 passed,
  zero failed**, including all four prior failures.
  Logs: `/tmp/rustycore-589-spell-handler-tests.log` and
  `/tmp/rustycore-589-spell-handler-tests-fixed.log`.
- Workspace format was applied before the final corrected handler run. Bot
  format was applied before its later evaluator/plan edits; recheck that surface
  on resumption. No services, DB operations, commits, push or merge occurred.

Resume only after the user's stop is lifted. Remaining local acceptance includes
the latest standalone bot tests, packet/domain suites, shared consumers and
production composition, plus reviewed ownership/persistence-reference and
physical-policy consistency. In particular, the new Session test registration
adds two lines above its existing physical ceiling; resolve this coherently
instead of blindly raising the ceiling. The syntax ownership baseline has not
yet been updated for the moved methods or new command. Full #589 acceptance and
fresh paired runtime scenarios remain outstanding.

## Remaining acceptance and limits

Tests have been authored for application phases, the inclusive 400 ms boundary,
replacement/revalidation, canonical allocation and reentry, recipient fences,
cancellation and late failure order. Typed payload serialization tests cover power,
runes, trajectory, immunities, prediction, target points and ammunition. The
packet crate's own tests remain **unexecuted**; serializer capability alone does not establish integrated
production behavior for every optional section.

The integrated bot now supports scripted cast/cancel/wait actions and observer
collection; [its guide](../../tools/wow-test-bot/CAST_LIFECYCLE.md) distinguishes
packet acceptance from unproven collection. The shared human/JSON report assembly
was moved to `src/run_report.rs`, keeping that responsibility together and
reducing the oversized main file. No live scenario has been executed. Final acceptance
must exercise these actions, verify packet fields/order/connections, run affected
shared-consumer and #587/#588 regressions, and review the ownership/physical policy
delta before recording results and the actual candidate identity.

The Player producer currently supplies the represented power section. Complete
rune/projectile behavior, Player advanced combat-log stat authority and general
UnitCondition-dependent visual evaluation are not established by this work.
Neither full target/effect execution parity nor the entire SpellHistory model is
claimed. Any such dependency needed by the approved acceptance cases must be
resolved before scoped acceptance, not waived by this list.

No publication, integration or runtime operations are recorded for #589.
Required #584 core continues before #583, then #153 and closure of #133.
