# Claude Porting Instructions

Este documento es un prompt/guia para continuar el port completo C++ -> Rust de RustyCore sin perder contexto ni cerrar gaps falsos. Pegalo en Claude o usalo como checklist al iniciar una sesion.

## Prompt Base Para Claude

```text
Estas trabajando en RustyCore, un port completo de un servidor TrinityCore-derived C++ a Rust. El objetivo es terminar el port completo, no hacer una demo, no cerrar una fase parcial con comportamiento aproximado y no "hacer que compile" a costa de perder paridad.

Rutas obligatorias:
- Repo Rust: /home/server/rustycore
- Fuente C++ canonica: /home/server/woltk-trinity-legacy
- Rama de trabajo: develop
- Docs de estado: /home/server/rustycore/docs/migration/current-session-handoff.md
- Inventario principal: /home/server/rustycore/docs/migration/inventory/r8-entities-miniphase.md
- TSV principal: /home/server/rustycore/docs/migration/inventory/r8-entities-miniphase.tsv
- Plan por fases (orden de ejecucion): /home/server/rustycore/docs/MIGRATION_ROADMAP.md
- Indice de modulos + estado/auditoria: /home/server/rustycore/docs/migration/_INDEX.md
- Auditoria honesta de progreso: /home/server/rustycore/docs/migration/honest-progress-audit.md
- Guia local de agente: /home/server/rustycore/CLAUDE.md

Estado actual conocido (puede haber avanzado; corre `git log` para confirmar HEAD real):
- HEAD reciente: af9ffe0 Align can_take_quest gate order to C++
- Ultima base funcional auditada del port: 1af9223 Add honest progress audit (R8-entities). Despues de 1af9223 se han revisado por el flujo analista->dev->revisor: b285e92 (#765 test-debt wow-map 614/0), af9ffe0 (#766 reorden de fidelidad de gates). ef9741f/12686e2 son documentacion.
- Progreso honesto documentado: 736/759 = 96.97% (sin cambio por #765/#766: son test-debt/fidelidad, no gaps nuevos). Ver `docs/migration/honest-progress-audit.md` para el desglose real (lo "atendido" no es lo mismo que runtime vivo).
- Ultimo item del handoff: FIDELITY / #NEXT.R8.ENTITIES.766.
- Para cambios funcionales, audita siempre los commits posteriores a 1af9223 contra C++; no aceptes nada por el mensaje.

Regla principal:
No confies en Rust, en summaries de IA ni en docs antiguas como prueba de paridad. Siempre contrasta contra /home/server/woltk-trinity-legacy antes de implementar o aceptar cambios.

Al arrancar:
cd /home/server/rustycore
git status --short --branch
git log --oneline --decorate -8
head -n 20 docs/migration/current-session-handoff.md
sed -n '1,220p' CLAUDE.md

Si hay commits nuevos despues de la base que conoces:
git log --oneline 1af9223..HEAD
git diff --stat 1af9223..HEAD
git diff 1af9223..HEAD

No aceptes commits nuevos solo por el mensaje. Audita cada cambio contra C++.
```

## Que Proyecto Es

RustyCore es un port Rust de un servidor privado WoW basado en TrinityCore. El C++ legacy en `/home/server/woltk-trinity-legacy` es la fuente de verdad para:

- orden de gates y validaciones;
- layouts de paquetes;
- orden de campos SQL;
- side effects de handlers;
- estado de jugador, quest, mapa, criatura, item, reputacion, spell, aura, etc.;
- comportamiento de runtime y fanout;
- casos donde se retorna silenciosamente vs se envia packet;
- diferencias entre estado canonical, session-local, DB y packet.

El objetivo final es port completo. Un slice puede ser `represented-partial`, pero debe dejar claro que sigue faltando para la paridad real.

## Regla Anti-Alucinacion

Claude debe tratar todo como no probado hasta verificarlo.

No hacer:

- No decir "ya esta portado" porque hay una funcion Rust con nombre parecido.
- No cerrar inventario por leer un comentario.
- No hacer bulk close de muchos `#NEXT` sin codigo y tests.
- No inventar rutas, structs o campos.
- No confiar en `MIGRATION_STATUS.md` si contradice handoff o codigo actual.
- No marcar manual-test-ready si no se ha probado manualmente con cliente/servidor instalado.
- No cambiar arquitectura runtime con big-bang rewrites.
- No usar TrinityCore 3.3.5 archived como fuente de codigo; solo puede ayudar con logica si el legacy local esta incompleto, y debe documentarse.

Si hay duda:

1. buscar en C++;
2. buscar en Rust;
3. comparar;
4. hacer cambio minimo fiel;
5. probar;
6. documentar boundaries.

## Flujo Obligatorio Por Cada Gap

1. Leer estado actual:

```bash
cd /home/server/rustycore
git status --short --branch
head -n 30 docs/migration/current-session-handoff.md
```

2. Elegir un gap real desde:

```bash
rg -n "remain open|Boundaries|Remaining gaps|SatisfyQuest|ConditionMgr|manual-test-ready|live-runtime" docs/migration/current-session-handoff.md docs/migration/inventory/r8-entities-miniphase.md
```

Para trabajo de runtime/arquitectura (Maps, MapManager, tick global, entidades canonicas vivas), el plan por fases vive en `docs/MIGRATION_ROADMAP.md` (Fase 0 = rehacer Maps, Fase 1 = entidades canonicas, etc.) y el estado por modulo en `docs/migration/_INDEX.md`. Ese trabajo es L3/L4 del roadmap; NO usar el namespace `#NEXT.R8.ENTITIES.*` para slices de runtime/arquitectura — usar la fase/modulo del roadmap. El namespace `R8.ENTITIES` es para la mini-fase de entidades representadas, no para la convergencia de runtime.

3. Localizar C++ exacto:

```bash
rg -n "NombreFuncion|CONDITION_X|HandlerName|CanTakeQuest|RewardQuest" /home/server/woltk-trinity-legacy/src
```

4. Leer C++ alrededor de la funcion, no solo una linea:

```bash
sed -n '14080,14110p' /home/server/woltk-trinity-legacy/src/server/game/Entities/Player/Player.cpp
sed -n '15080,15230p' /home/server/woltk-trinity-legacy/src/server/game/Entities/Player/Player.cpp
```

5. Localizar Rust equivalente:

```bash
rg -n "can_take_quest|satisfy_quest|represented_.*like_cpp|ConditionType" crates
```

6. Comparar orden, inputs, outputs y side effects:

- Que datos lee C++?
- Que datos muta C++?
- Que packets envia?
- Que DB statements emite?
- Donde retorna false/silencioso?
- El Rust tiene esas mismas fuentes de datos?
- Si no existen, hay que crear store/campo/bridge o dejar boundary explicito?

7. Implementar un slice pequeno y verificable.

8. Anadir tests. Idealmente:

- una rama positiva;
- una rama negativa;
- un test de orden si el C++ depende del orden;
- un test de no-side-effect si C++ retorna silenciosamente.

9. Actualizar docs:

- `docs/migration/current-session-handoff.md`
- `docs/migration/inventory/r8-entities-miniphase.md`
- `docs/migration/inventory/r8-entities-miniphase.tsv` si corresponde

10. Validar:

```bash
cargo fmt --check
PROTOC=/home/cdmonio/.local/protoc/bin/protoc cargo test -p wow-world --lib
PROTOC=/home/cdmonio/.local/protoc/bin/protoc cargo check -p world-server
git diff --check
awk -F '\t' 'NF != 9 { print FNR ":" NF ":" $0; bad=1 } END { if (bad) exit 1; print "TSV_OK" }' docs/migration/inventory/r8-entities-miniphase.tsv
```

No siempre hace falta `cargo test -p wow-world --lib` completo si el cambio esta en otro crate, pero debe haber un test focalizado y un check del crate/ruta afectada. Si toca `world-server` o recursos de sesion, ejecutar `cargo check -p world-server` con `PROTOC`.

11. Commit y sync solo si el slice esta estable:

```bash
git add <files>
git commit -m "Short faithful message"
git push origin develop
git checkout main
git merge --ff-only develop
git push origin main
git checkout develop
git status --short --branch
```

## Ejemplo Concreto De Como Hacer Un Port

Caso ejemplo: portar un gate faltante de `Player::CanTakeQuest`.

### 1. Encontrar C++

```bash
rg -n "CanTakeQuest|SatisfyQuestBreadcrumbQuest" /home/server/woltk-trinity-legacy/src/server/game/Entities/Player/Player.cpp
sed -n '14090,14105p' /home/server/woltk-trinity-legacy/src/server/game/Entities/Player/Player.cpp
sed -n '15080,15210p' /home/server/woltk-trinity-legacy/src/server/game/Entities/Player/Player.cpp
```

Anotar:

- orden exacto de `CanTakeQuest`;
- helper exacto;
- que parametros usa;
- si usa quest store, player quest status, rewarded quests, daily/weekly/monthly state, conditions, expansion, skill, reputation;
- que pasa si falta metadata.

### 2. Encontrar Rust

```bash
rg -n "fn can_take_quest|satisfy_quest|CanTakeQuest|Breadcrumb" crates/wow-world/src crates/wow-data/src
```

### 3. Comparar

Ejemplo de tabla mental:

| C++ gate | Rust existe | Rust esta cableado | Gap |
| --- | --- | --- | --- |
| `SatisfyQuestPreviousQuest` | si | si | no |
| `SatisfyQuestBreadcrumbQuest` | helper parcial/no | no | si |
| `SatisfyQuestDependentBreadcrumbQuests` | si | si | no |

### 4. Implementar

Seguir orden C++ exacto. Si C++ hace:

```cpp
return SatisfyQuestStatus(qInfo, msg)
    && SatisfyQuestExclusiveGroup(qInfo, msg)
    && SatisfyQuestClass(qInfo, msg)
    && SatisfyQuestRace(qInfo, msg)
    && SatisfyQuestLevel(qInfo, msg)
    && SatisfyQuestSkill(qInfo, msg)
    && SatisfyQuestReputation(qInfo, msg)
    && SatisfyQuestDependentQuests(qInfo, msg)
    && SatisfyQuestTimed(qInfo, msg)
    && SatisfyQuestDay(qInfo, msg)
    && SatisfyQuestWeek(qInfo, msg)
    && SatisfyQuestMonth(qInfo, msg)
    && SatisfyQuestSeasonal(qInfo, msg)
    && SatisfyQuestConditions(qInfo, msg)
    && SatisfyQuestExpansion(qInfo, msg);
```

Rust debe preservar el orden en `can_take_quest`. No meter el nuevo gate al final si C++ lo ejecuta antes, salvo que documentes que es equivalente y lo pruebes.

### 5. Test

Crear test que falle antes del cambio y pase despues:

- quest debe bloquearse si el prerequisito no esta satisfecho;
- quest debe aceptarse si el prerequisito si esta satisfecho;
- si hay store ausente y el patron actual del port es fail-open para esa metadata, testearlo o documentarlo.

### 6. Documentacion

Agregar item nuevo:

```text
#NEXT.R8.ENTITIES.xxx — represented-partial C++ <funcion/gate> now ...

C++ anchors: /home/server/woltk-trinity-legacy/...
Rust targets: crates/...
Acceptance: ...
Checks: ...
Boundaries: ...
Current represented/closed inventory count is N/M = P%.
```

En TSV mantener 9 columnas.

## Arquitectura Runtime Actual

No asumir una sola fuente de verdad de mapa/criaturas. Actualmente hay tres modelos:

1. Legacy `wow_world::MapManager`
   - compartido entre sesiones;
   - AI/combat representado via ticks de sesion;
   - sin reloj global propio.

2. Canonical `wow_map::MapManager`
   - tick global alrededor de 10ms;
   - estructura de update tipo C++;
   - todavia no despacha AI/combat real en creature runtime.

3. Global world loop
   - tickea canonical `wow_map::MapManager`;
   - no tickea legacy `wow_world::MapManager`.

No existe ya el antiguo campo `WorldSession.creatures: HashMap<...>` que algunos docs viejos mencionaban. No construir nada nuevo sobre esa premisa.

Roadmap runtime incremental:

1. Caracterizacion actual hecha en `#NEXT.R8.ENTITIES.764`.
2. Dar reloj sessionless al legacy map.
3. Fanout de movimiento desde global tick mediante registry por mapa.
4. Resolver combat en global clock una sola vez.
5. Unificar respawn.
6. Fuente unica de criaturas metodo a metodo.
7. `SendObjectUpdates`, scripts, weather, threat y fanout restantes.

No hacer big-bang. Si un cambio toca runtime global, aislarlo con regression tests y pedir confirmacion si hay riesgo de doble combat/tick/fanout.

## Estado Actual Y Gaps Repetidos

Estado documentado (puede haber avanzado; confirmar con `git log`):

- HEAD reciente: `af9ffe0 Align can_take_quest gate order to C++`
- base funcional auditada: `1af9223 Add honest progress audit (R8-entities)` (commits posteriores revisados: `b285e92` #765, `af9ffe0` #766)
- progreso: `736/759 = 96.97%`
- `wow-map --lib`: limpio `614/0`
- `wow-world --lib`: limpio `1051/0`

Gaps abiertos repetidos en handoff:

- full `ConditionMgr` target/searcher/map/world-state/active-event coverage;
- `Player::SatisfyQuestBreadcrumbQuest` recursivo;
- `SatisfyQuestTimed`, day, week, month en accept;
- GM override visibility / server-side visibility;
- AI override dialog status;
- battleground chest `CanActivateGO`;
- live-runtime / MapManager tick integration;
- install/restart/manual-test-ready real.

Esta lista no es exhaustiva. Siempre leer `current-session-handoff.md`.

## Como Auditar Trabajo De Otra IA

Si hay commits nuevos:

```bash
git log --oneline 1af9223..HEAD
git diff --stat 1af9223..HEAD
git diff 1af9223..HEAD
```

Para cada commit:

1. Identificar que dice cerrar.
2. Ver si toca codigo real o solo docs.
3. Buscar C++ equivalente.
4. Comparar orden/logica/side effects.
5. Ejecutar test focalizado.
6. Revisar docs e inventario.
7. Si falta algo, corregir antes de seguir.

No aceptar un commit porque "parece razonable". Aceptarlo solo si la evidencia lo prueba.

## Checklist De Cierre

Antes de decir "cerrado":

- [ ] C++ refs exactas localizadas.
- [ ] Rust comparado contra C++.
- [ ] Codigo implementado con nombres/orden `*_like_cpp` si aplica.
- [ ] Tests focalizados pasan.
- [ ] `cargo fmt --check` pasa.
- [ ] `world-server` check pasa si aplica.
- [ ] `git diff --check` pasa.
- [ ] Docs actualizadas.
- [ ] TSV 9 columnas si se toca TSV.
- [ ] Boundaries restantes claros.
- [ ] Progreso recalculado sin inflar.
- [ ] Commit/push/sync main solo si esta estable.

## Respuesta Esperada De Claude Al Trabajar

Claude debe reportar en cada cierre:

```text
Implementado:
- ...

C++ contrastado:
- ruta:lineas

Rust tocado:
- ...

Tests:
- comando exacto: resultado

Docs:
- item #NEXT...

Progreso:
- N/M = P%

Boundaries:
- ...
```

Si no ha terminado, debe decir que queda pendiente y no presentar el slice como completo.
