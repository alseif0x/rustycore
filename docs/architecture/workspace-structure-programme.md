# Programa maestro de estructura del workspace

**Para qué sirve:** índice, orden de ejecución y estado del trabajo estructural.
Fija dependencias y evidencia de cierre para retomar sin perder contexto.

**Para retomar:** leer primero el [estado de continuación](#estado-de-continuacion).
El [historial separado](workspace-structure-history.md) conserva intentos y resultados;
no acredita aceptación de la punta actual.

**Autoridades (no compiten entre sí):**

| documento | gobierna |
|---|---|
| [structure-and-conventions.md](structure-and-conventions.md) | el **estándar**: capas, nombres, visibilidad, tests, presupuestos, checklist |
| [wow-world-distribution-plan.md](wow-world-distribution-plan.md) | el detalle de **wow-world** (forma objetivo y fases F0-F13) |
| este documento | el **orden, dependencias y estado** de todo el workspace |
| [refactor-completion-plan.md](refactor-completion-plan.md) | el plan técnico general del port (no lo sustituimos) |

**Dos planos:** C++ 3.4.3 fija comportamiento; este programa decide la estructura.

## 1. Estado de partida (auditoría medida, `3.4.3` + rama de distribución)

Reejecutable con el script de auditoría (mide líneas, ficheros, fichero mayor, deps internas,
consumidores, capa y violaciones) sobre `crates/`.

**Monolitos:** `wow-world` 410 725 (mayor 1 720) · `wow-data` 83 539 (1 999) · `wow-entities`
79 731 (**4 726**) · `world-server` 60 368 (**5 675**) · `wow-map` 54 927 (**5 133**) ·
`wow-packet` 50 339 (1 682) · `wow-database` 45 556 (1 793) · `wow-social` 5 354 (2 508).

**Muertos/stubs/mal declarados:** `wow-spell` (1 línea) · `wow-pvp` (1) · `wow-achievement` (1) ·
`wow-scripts` (44, depende de `wow-script`) · `world-modules` (23, **depende de `world-server`**) ·
`rustycore-db` (295, sin consumidor) · `wow-collections` (693, sin consumidor y **colisiona de
nombre** con el dominio de colecciones de cuenta) · `wow-session` (596, capa ambigua) ·
`capture-diff` (herramienta de QA dentro de las capas de juego) · `wow-recastdetour` (vendor sin
señalizar) · `wow-chat` (solapa con `wow-social`).

**Inversiones de capa:** `data→entities`, `data→movement`, `entities→loot`, `map→loot`,
`packet→loot`, `packet→movement`, `database→persistence`, `world-modules→world-server`, y las
laterales `ai→instances`, `conditions→loot`, `scripts→script`.

## 2. Secuencia maestra

Orden por dependencias reales: cada fase espera su fila, no toda la ola anterior (§4).
Owners y archivos distintos avanzan en paralelo; la aceptación conserva un solo ejecutor.

### Ola A0 — endurecer el estándar antes de tocar crates
Contraste con proyectos grandes de Rust (`rustc`, `rust-analyzer`, `bevy`, `polars`, `tokio`,
`datafusion`): el plan va en la dirección correcta, pero conviene añadir tres piezas antes de
mover crates, porque son más baratas ahora que después.

| id | objetivo | evidencia de cierre | depende de |
|---|---|---|---|
| A0.1 | **Decidir el mapa de *features***: qué crates/subsistemas son opcionales (`world-modules`, scripting, anticheat) y garantizar que el build base no los requiere. Cargo unifica features por crate en todo el workspace, así que la decisión afecta a caché y a compilación | el build base compila sin features opcionales; decisión escrita en el estándar | — |
| A0.2 | **`xtask` del workspace** que aloja los comandos de estructura: auditoría de crates, chequeo de aristas de capa, regeneración revisada de políticas | `cargo xtask structure-audit` y `cargo xtask check-layers` en verde | — |
| A0.3 | **`[workspace.lints]`** (clippy/rustc) y política por crate (`unsafe`, `missing_docs` en API pública); los warnings dejan de ser decorativos | lints activos; ningún crate afectado gana warnings nuevos | — |
| A0.4 | **Higiene de grafo con herramientas estándar**: `cargo-machete` (dependencias no usadas), `cargo-deny` (licencias/avisos/versiones duplicadas), `cargo tree -d` | informe inicial registrado y deuda adjudicada a A1/C/D | — |
| A0.5 | **ADRs** de la decisión estructural (alternativas y consecuencias) y **marcar el código vendido** (navmesh) como exento de presupuestos y lints | ADR enlazado desde el estándar; crates vendor señalizados | — |
| A0.6 | **Presupuesto de documentación**: el estándar se mantiene corto (es regla); el histórico va a ADRs, no a un plan que crece | techo de tamaño para documentos de arquitectura | — |
| A0.7 | **Herramientas opcionales a decidir**: `cargo-nextest` (ejecución de los ~3 900 tests) y `cargo-public-api` (snapshot de la superficie pública de `wow-entities`, `wow-map`, `wow-packet`); `cargo-hakari` **solo** si la medición de build demuestra duplicación de features | decisión escrita; si se adopta, comando en el `xtask` | A0.2 |

**Lo que NO copiamos** (y por qué): el troceo a escala `bevy`/`zed` (cientos de crates) porque
aquí no hay ecosistema de plugins — 12-14 dominios más directorios es la medida; el
feature-gating de todo; y `cargo-hakari`/workspace-hack antes de medir, porque añaden complejidad
que hay que justificar con números.

### Ola A — saneamiento barato (no cambia contratos ni comportamiento)
| id | objetivo | evidencia de cierre | depende de |
|---|---|---|---|
| A1 | plegar `rustycore-db` en `wow-database`; retirar `wow-pvp`, `wow-achievement`, `wow-scripts` (o consolidar en `wow-script`); renombrar la utilidad `wow-collections`; decidir `wow-session` y `wow-chat` | workspace compila; recuento de crates baja; `inventory::submit!` y nº de tests invariantes | A0 |
| A2 | invertir `world-modules` para que no dependa de `world-server` | grafo sin esa arista | A1 |
| A3 | marcar tooling: `capture-diff` fuera de las capas de juego; renombrar el vendor de navmesh | grafo y capas coherentes con el estándar | A1 |

### Ola B — `wow-world` (detalle en su plan)
| id | objetivo | depende de |
|---|---|---|
| B1 | F5 ✅ `misc` desmontado en 13 dominios (commit `e719ac38`) | — |
| B2 | F6 dominios cohesivos a su crate (`reputation`, `planner`, `profession`/`trainer`, `entity_update_bridge`, `battle_pet_*`) | B1 |
| B3 | F10a: suites de aplicación a targets de integración de `wow-world`, con fixtures mínimas; pruebas de reglas en su dominio (reformulación del 2026-09-25) | B2 |
| B4 | F7 sub-estados dueños dentro de `WorldSession` (**diseño**) | B2 |
| B5 | F8 handlers grandes → adaptadores ≤600 líneas (**diseño**) | B4 |
| B6 | F9 `map_manager` + `map_manager_tests` → `wow-map` con contrato (**diseño**) | B4 |
| B7 | F10b resto de tests; F13 warnings y política de ownership con delta revisado; consumidores de fixtures pendientes de B3 | B3 (residual), B5, B6 |

### Ola C — inversiones de capa (cada una es un movimiento pequeño)
| id | objetivo | depende de |
|---|---|---|
| C1 | modelos inmutables compartidos a `wow-data-model`; loaders/stores quedan en `wow-data`; retirar `data→entities/movement` sin introducir `entities/movement→data` productivo | A1 |
| C2 | los tipos que `entities`/`map`/`packet` necesitan de loot suben a su dueño; se rompen `entities→loot`, `map→loot`, `packet→loot` | C1 |
| C3 | retirar `packet→movement` trasladando su conversión al adaptador de aplicación; preservar `database→persistence` como adaptador que implementa el puerto | C2 |
| C4 | decidir las laterales vigentes (`ai→instances`, `conditions→loot`) por sus consumidores; retirar del baseline la histórica `scripts→script` ya inexistente | C3 |

### Ola D — los siguientes monolitos (mismos presupuestos del estándar)
| id | objetivo | prioridad | depende de |
|---|---|---|---|
| D1 | **`wow-entities`**: fichero mayor 4 726 → ≤1 000; separar modelo canónico de estado de gameplay | alta | C2 |
| D2 | **`world-server`**: `app.rs` 5 675 → módulos por fase; composición delgada | alta | B7 |
| D3 | **`wow-map`**: ficheros 5 133 → ≤1 000; separar runtime de grillas de la fachada | alta; adelantar con B6 | dueño y contrato de B6 fijados; cierre en paralelo |
| D4 | **`wow-data`** y **`wow-database`**: techos de fichero y organización por catálogo/tabla | media | C1 |
| D5 | **`wow-packet`** y **`wow-social`**: techos de fichero; packet solo wire | media | C3 |
| D6 | **Producto de módulos (ADR-002), asignado a #583**: contrato de datos versionado en `wow-module-api`, adaptadores nativo y Wasm sobre el mismo contrato, `wasm-runtime` opcional, límites de ejecución y aislamiento de fallos; incluye la entrega Rust/Wasm/C exigida por el plan de módulos | tras el núcleo #584 | requisitos core de #584 y A0.1 |

### Ola E — cierre
| id | objetivo | depende de |
|---|---|---|
| E1 | auditoría del estándar en todo el workspace existente (mismo script): 0 stubs, 0 sin consumidor, 0 inversiones, 0 fichero > 1 000 o excepciones específicas revisadas según la política | A-D del núcleo #584; D6 se acepta en #583 |
| E2 | ledger y política físicas consistentes con el árbol (delta revisado) | E1 |
| E3 | **validación final única** (`./tools/validation-v2 final --architecture`) y evidencia publicada | E2 |
| E4 | medición de build y latencia de herramientas antes/después (#1231); se publica tal cual, incluso si es negativa | E1 |

## 3. Registro de estado

Se actualiza en el commit de cada cierre: `[ ]` pendiente, `[~]` en curso, `[x]` cerrada con commit.

```
A0.1 [x]  A0.2 [x]  A0.3 [x]  A0.4 [~]  A0.5 [x]  A0.6 [x]  A0.7 [x]   <- ola A: PUERTA VERDE
A1 [x]  A2 [x]  A3 [x]
B1 [x] e719ac38   B2 [x] 98c5b14a   B3 [~]  B4 [~]  B5 [~]  B6 [~]  B7 [~]
C1 [ ]  C2 [ ]  C3 [ ]  C4 [ ]
D1 [ ]  D2 [ ]  D3 [ ]  D4 [ ]  D5 [ ]
E1 [ ]  E2 [ ]  E3 [ ]  E4 [ ]
```

<a id="estado-de-continuacion"></a>

### 3.1 Estado de continuacion (actualizado 2026-09-30)

Esta entrada se mantiene al avanzar; sustituye las conclusiones contradictorias de las notas
históricas para seleccionar el siguiente trabajo. Los SHA antiguos se conservan como evidencia
histórica: la reescritura de mensajes cambió los identificadores, no autoriza a relabelar pruebas.

**Árbol activo observado el 2026-09-30:** worktree `/home/server/rustycore-world-refactor`,
rama `584-map-manager-domain`, HEAD `d6c92488`; base `3.4.3` en `e786ece1`.
Hay 213 commits por delante del remoto de la rama y una barrida sin commitear ni validar.
El [punto de partida](workspace-structure-history.md#estado-de-git-al-iniciar-el-programa)
conserva el contexto anterior. Reconsultar Git/procesos al retomar; no hay autorización de push.

| pieza | estado comprobado y límite |
|---|---|
| B6 | `map_manager` y su suite están en `wow-map`; comparación estática conserva 136 anotaciones de test. Se estrecharon exports internos revisados de terrain, grid, pathfinder, respawn y runtime_state; el manager legado sigue requerido por `wow-world`/`world-server`. La barrida D3 deja `map/mod.rs` en 714 líneas, con children privados menores de 1 000, sin aceptación. La división física no retira los dos managers, productores de ticks ni sus puentes. Faltan el cierre semántico, features/composición y aceptación del candidato. |
| B5 | Raíces de loot y character miden 243/239 líneas y todos sus hijos extraídos menos de 600. Se estrecharon rutas heredadas de visibilidad de ambos adaptadores, incluido el plan de inventario test-only tras mover sus dos tests privados. Faltan el resto de exports y la aceptación del candidato. |
| B3 | `group_tests` ya es target de integración. Quest: referencia histórica 198; 16 originales state/persistence/objectives externos conservan nombres y secuencia de 62 macros assert frente a HEAD. Los tres nuevos de hydration están externos. Otros once originales/36 assertions: Packet cinco, Model uno, Progression dos y APP tres; parent confirma nueve cuerpos literales y dos con tokens/literales idénticos ignorando whitespace, y los 16 métodos Template literales frente a PRE íntegro. Fixture mutable/party opt-in y cuatro casos de wiring conservan la vía productiva normal con feature. Residuales handlers Quest entregados: 21/74, 14 cuerpos idénticos frente a PRE íntegro y siete con acceso adaptado; mismos nombres/recuentos, tres assertions usan la observación del mismo owner. Controller opt-in 72/tres nuevos13 conserva attach normal con feature; PRE de state es parcial y no prueba todos sus campos. Quest físico y última suite entregados: 65 archivos ≤600 (máximo545), 14 originales/86 assertions trasladados; parent confirma sus nombres/recuentos, lee los 14 casos y las fachadas. No quedan casos/montajes directos en las dos familias Quest; reconciliación histórica y aceptación pendientes. Character: montaje previo observado 41 externos, 253 privados y uno vendor separado; 303/107 históricos requieren reconciliación por nombre. Entregada catalog-query completa: once originales/33 assertions, child productivo 261 y fixture 55 revisados en fuente; secuencia de 33 macros assert idéntica frente a HEAD; RawCache conserva dispatch registrado y TactKey instala el mismo Arc. Inventario: 15 originales/34 assertions entregados y fachadas/admisión revisadas en fuente; quedan 49 asignados. Opt-in privado explícito para snapshots ownerless aprobado, default deshabilitado con feature y siempre fail-closed ante handle observado/rechazado; cfg(test) conserva su fallback previo. Loot: referencia 266/1 372; 151 privados, 102 externos y 13 de dominio tras money19; APP suma 817/489 assertions (privados/externos); parent confirma ledger residual 151/817. Traslados de 14/62 y 15 durable-claim/64; parent conserva los 15 nombres y recuentos de assertions contra HEAD, fachada usa JoinHandle original. Restaurados nombres/assertions/input cap; revisión completa/features pendientes. Cuatro fixtures conservan 36 llamadas externas/una interna; money19 completo 19/66; 17 nombres/recuentos contra HEAD y dos personales/11 revisados contra PRE, conserva 999 global/123→0 owner/456 otro. Ciclo open/sync/consume usa opt-in por llamada; parent revisa guards y wrappers normales contra PRE. Negativos funcionales de feature normal y autoridad observada/rechazada, más barrida residual151, asignados; sin ensanchar fallback productivo. Sin compilación ni aceptación. |
| B7 | Hay limpieza parcial local, incluidos siete imports sin uso retirados el 2026-09-29; los consumidores restantes de fixtures dependen de B3. No está cerrado. |
| Talentos #578 | La rama `recover/578-talent-catalog`, `0845f5b3`, conserva `docs/migration/recovered/578-talent-catalog-2026-09-04.patch`; el archivo solo existe en esa rama, no en este checkout. No está aplicado; preservar la rama y adaptar por consumidores actuales cuando corresponda. |
| Orquestación | GPT-6.1 Sol high / GPT-6 Luna max configurados en este worktree y el checkout principal por petición del usuario. Los workers nativos se lanzan con modelo y esfuerzo explícitos; el fichero de configuración por sí solo no demuestra el runtime de una sesión ya abierta. Cambios compartidos aún locales y `.codex/config.toml` ignorado. Preservar estos cambios al continuar. |

**Actualización de inventario (2026-09-30, posterior a la fila B3):** Pauli entrega
64/64 originales externos y retirada de nueve archivos/montajes privados. Parent
compara PRE íntegro en `.agent-artifacts/character-inventory-pre-64`: 64 nombres
únicos, sin faltantes/duplicados, 203 assertions de escenario con la misma secuencia
de macros; 20 cuerpos literales y 44 adaptados. Los otros tres macros pertenecen
a helpers (206 total); los cinco nuevos/26 quedan aparte. Parent lee fachadas,
forwards de handlers, opt-in/capacidades, gates de equipo/modificadores/diagnóstico
y cinco casos de frontera. Feature normal mantiene opt-in deshabilitado; cualquier
handle impide fallback. El PRE se materializó del snapshot anterior, sin SHA
capturado. Parent lee el diff completo de los 44 adaptados y sus fachadas;
aceptación pendiente. Barrida física de ocho hotspots player_items y cinco
Character asignada, sin mudar autoridad ni acreditar reducción global;
esta entrega sustituye «quedan 49 asignados», no acredita verde.

**Retirada de hechizos entregada (2026-09-30, posterior a la fila B4/combat):**
la operación completa frames/seen pertenece a PlayerSpellRuntimeState; APP
conserva catálogos lazy, guards/invalidaciones, skills/equipo y publicación. Parent
lee PRE y operación/protocolo/owner/adaptador completos, nueve tests de contrato,
los nueve originales adaptados y sus fixtures, y los dos nuevos APP. Comparación
independiente: 70 métodos anteriores del runtime y 29 restantes del spellbook
literales; nueve originales Domain/21 assertions; 19 APP/77 assertions literales
y archivos scenario2/3 enteros idénticos. Runtime/root 528, traits 168, operación
266 y APP adapter 141; nueve Domain/40 y dos APP/13 nuevos sin ejecutar. PRE íntegro
en `.codex/spell-unlearn-presnapshot-20260930`; sin aceptación. La suite amplia de
spell-state y su distribución siguen pendientes; se explora su siguiente barrida.

**Retirada B4 entregada, sin aceptación:** validación causal completa de adquisición
pasa a `wow-spell-acquisition`, incluidos replay/identity/provenance, publicaciones
y evidencia por ocurrencia. Parent compara independientemente los once cuerpos
trasladados: idénticos salvo nombres/error de dominio; lee la conversión de los
24 errores y los 21 casos nuevos. Siete originales/nueve assertions pasan a Domain;
cuatro APP/catorce assertions permanecen en World. APP conserva profesión, autoridad
actual, durabilidad/COMMIT, instalación y publicación. Reducción de esta familia:
837 líneas de producción y 324 de tests, 1 161 netas de World. PRE y entrega íntegros
en `.codex/spell-acquisition-validation-presnapshot-20260930`. Parent confirma
además cinco cuerpos de prepare, dos de translate y profesión idénticos salvo
el callee trasladado, cuatro originales APP literales (7/1/4/2 assertions) y
model/fixture/montajes de tests anteriores byte-idénticos. Falta aceptación final.

D2 compositor tenía raíz 792 natural/48 children ≤453; la continuación entregada
reduce app.rs a 250 y world_startup.rs a 627 tras extraer account admission
(69 líneas, tres referencias/cinco resultados existentes). Parent lee el diff
completo de account admission y su locator, y confirma independientemente el
cuerpo trasladado y la función completa de continuación al expandir la llamada:
son literales, incluido whitespace, en
`/tmp/sol-account-admission-source-comparison-20260930.json`. El child sigue 27
líneas sobre 600; no se acredita cierre del presupuesto. Entregas/source-only en
`/tmp/luna-d2-continuation-94c4jf1w/` y
`/tmp/luna-d2-account-admission-0vodt6au/`; aceptación pendiente.
Banach corrige los dos locators hotfix y el helper de localized strings
que tenía await sin declaración async. Su comparación de operaciones usa children
actuales en ambos lados cuando no hay PRE de esos children: no prueba equivalencia
independiente de todos sus cuerpos. Parent lee ambos locators pendientes de
DungeonEncounter/Gossip, ya corregidos, y sus child callers: conserva los seis/tres
hechos originales y añade unicidad/orden en cada fuente real. El locator Gossip
ya fallaba en ambos PRE. No aceptar el viejo root600 comprimido
ni crear una fábrica universal para cumplir la cifra. Informe de revisión de fuente
en `/tmp/luna-d2-closure-source-review.md`, sin pruebas ejecutadas.

El corte adicional propuesto de formación del mundo (20 referencias/nueve
resultados existentes, root565–621) sólo ahorra 23 líneas; no se implementa para
cumplir una cifra que seguiría abierta. WorldStartup627 sigue pendiente de un límite/corte
semántico aplicable; no relajar el estándar ni inventar contexto universal.
Banach termina la preparación de B3 Character lifecycle/account enumeration/
world-entry y recibe implementación conjunta de 86 nombres APP: 64 Character/
240 assertions y 22 consumidores Session/113. PRE y contrato íntegros en
`/tmp/luna-b3-character-map-1_3netnq/`; target propio `character_lifecycle` y
fixture child separado, sin tocar los mounts Character de Pauli. Conserva
helpers/negativos/fences, rename12 y source guards; excluye inventory/catalogs,
collections y Spell/Skill. Parent recupera el baseline nominal 303 desde
origin/3.4.3=e786ece1a554724351307bfd4e05e63a89962b60: 33 fuentes completas y
303 nombres únicos en `/tmp/sol-character-reference-origin-20260930/`.
Snapshot concurrente de definiciones actuales: 239 tienen un locator y 64 tienen
dos (original Character y copia de la entrega lifecycle en curso); ninguno
ausente. Esto resuelve la falta del baseline nominal, no acredita mounts activos,
assertions ni aceptación; reconciliar cada destino y el significado histórico
de 107 sigue pendiente. No compilación ni tests ejecutados.

**Character lifecycle86, seam de fixture pendiente de implementar:** los 86
destinos y helpers están preparados, originales aún retenidos. Informe completo
`/tmp/luna-b3-character-map-1_3netnq/IMPLEMENTATION-WIP.md`: 85 cuerpos declarados
con tokens conservados tras rutas explícitas; uno mueve su arrange canónico y
observa la proyección después de soltar el guard, sin await. Las 358 observaciones
incluyen `matches!`, por lo que no sustituyen el anterior recuento de asserts.
La librería de un target externo no tiene `cfg(test)`; wrappers por sí solos no
reproducen los antiguos fixtures handleless. Banach recibe implementación del
modo CharacterLifecycle privado, opt-in explícito/default false, que reutiliza
las ramas/estado originales sólo en los casos seleccionados. Some handle, incluso
stale/rechazado, impide fallback; construcción normal con feature sigue igual.
Sin owner automático, gameplay duplicado ni zeros/Vec vacíos de sustitución.
ACK explícito recibido de Pauli, Einstein y Loot: state/construction libres, sin
patches en vuelo. Banach recibe turno exclusivo para integrar los dos roots y
completar helpers/delegates propios. La migración no se cuenta terminada hasta completar
este rail y retirar los 86 registros originales. Ninguna aceptación ejecutada.

**Character86 entregado después de integrar el rail:** Banach confirma libres
state/construction y entrega `/tmp/luna-b3-character-map-1_3netnq/DELIVERY.md`.
Parent lee el informe: 86 originales externos y cuatro negativos adicionales,
19 suites/19 mounts retirados y seis suites mixtas editadas, 19 excluidos
conservados. Conserva PRE íntegros antes del turno compartido y retirada; declara
85 cuerpos equivalentes bajo rutas explícitas y un arrange/save extraído.
Las 358 observaciones incluyen matches; la comparación independiente del rail,
campos/initializer, helpers y retirada queda pendiente. Flag privado opt-in
defaultfalse observado en construcción/state; no se declara aceptación.
Parent aplica después el snippet exacto Loot criteria en los roots liberados:
import/type/field/default disponibles sólo con cfg(test) o test-fixtures.
No ensancha grant stubs/encounter fallback ni cambia política productiva.
Banach implementa el corte semántico pendiente de app.rs D2 (792), después de
la lectura completa parental del compositor y la propuesta: continuación desde
world_templates hasta serve, con referencias concretas y owners anteriores
retenidos en root. LoginDatabase y Graveyard usan slots Option tomados sólo en
sus puntos originales de conversión Arc; se conserva el orden de drop ante error.
Hotfix delivery se extrae como loader secuencial completo. El presupuesto del
child sigue pendiente: no se comprime ni se acepta otro contexto universal.
Parent contrasta independientemente los 86 casos con PRE: 85 cuerpos coinciden
tras las rutas explícitas de símbolos, attachment, namespaces y seis observaciones
de fields. Lee el caso save completo, su arrange trasladado y la proyección
inmutable de cuatro valores; conserva los tres outcomes y assertions originales.
Las 353 macros assert y cinco matches se conservan por caso. No quedan los 86
nombres en los originales ni las 19 suites retiradas; imports/montajes de destinos
observados, con test-fixtures habilitado por la dev-dependency de World.
State/construction conservan tokens de fields/initializers tras excluir cfg,
flag privado/defaultfalse y agrupación de imports; ese contraste no demuestra
los gates ni typing. Parent lee los cuatro negativos nuevos completos. Informes
independientes `/tmp/sol-character-86-case-source-comparison-20260930.json`,
`/tmp/sol-character-86-retirement-source-comparison-20260930.json` y
`/tmp/sol-character-shared-root-source-comparison-20260930.json`. Revisión completa
de los forwards/rail y reconciliación303/107 aún pendientes, sin ejecución.

**Respawn entregado (2026-09-30, corrección Route A entregada sin aceptar):**
Huygens entrega store tagueado común, transporte move-only, lifecycle Actor y
consumidor APP preparados; inventario en `/tmp/luna-respawn-delivery-20260930.tsv`
y PRE sustantivos en `/tmp/luna-respawn-pre-20260930`. Escribe 33 pruebas nuevas
sin ejecutarlas. Parent confirma 17/18 cuerpos Map literales y el selector
Catalog como único delta restante; ocho cuerpos Pending, lifecycle Session y
siete funciones APP literales. Lee store/transport completo, pipeline Actor,
APP y 33 casos nuevos. Permanecen dos instancias temporales
del mismo tipo hasta transportar bajo quiescencia y retirar writers/borrowers
legacy. No hay activación ni autoridad productiva única acreditada. La vista
de rows pasa del orden HashMap no especificado a orden por key; Actor conserva
Instant/ordinal. El baseline de `session/mod.rs` es reconstruido quitando el
reexport nuevo, no un PRE original. Hallazgo abierto: APP ground-snap llama
`static_height_like_cpp` bajo guard canónico; `terrain.rs:107/with_tile` puede
leer tiles con `std::fs::read`; la factory también inicializa RNG. Parent fija la
continuación owned ligada al MapTickPlan real, con query/factory fuera del guard,
orden por mapa/ordinal y gates de lifetime/tail/transfer. Huygens entrega Route A;
admission canónica usa el loader Noop. Parent lee manager/request/reply/dispose,
Map prefix/guards, reservas checked, transferencia, gates de lifetime/tick y APP
driver, más sus 19 casos nuevos (7 manager, 8 ownership, 4 APP query). Compara
independientemente las 78 assertions literales de los 15 casos adaptados; los 18
restantes se declaran conservados. PRE específico de la corrección en
`/tmp/luna-respawn-io-pre-20260930`, inventario de 20 rutas en
`/tmp/luna-respawn-io-delivery-20260930.tsv`. Módulos productivos nuevos ≤246;
state compartidos existentes 783/937 antes del último fixture.

Hallazgo adicional corregido: el helper APP del original stale devolvía Vec vacío
antes de pasar por el driver real; assertions intactas no preservan esa cobertura.
Parent lee la corrección limitada: fixture feature-only cambia sólo incarnation
de un participante existente en un plan realmente admitido, conservando origin,
epoch, diff, keys y orden; el helper debe llamar siempre al mismo driver y su skip
stale productivo. Método 12 líneas en state_2 (949), APP tests177; original stale
mantiene seis assertions y se añade un caso que atraviesa directamente el driver
y observa cola, reservas, timer, resumption y abandono. PRE adicional íntegro en
`/tmp/luna-respawn-stale-fixture-pre-20260930`. No amplía normal API, fields o
fallback. Contrato completo en
`workspace-structure-b6-execution.md`; Melee se prepara en lectura, sin ejecutar
checks ni activar productores. La retirada legacy y aceptación siguen pendientes.

**Loot B3 residual entregado parcialmente:** Zeno entrega 87/151 originales,
494/817 assertions (81 APP/470 y seis Domain/24); 64/323 siguen privados con
cuerpos declarados byte-idénticos al PRE. Money19 conserva 19/66 y añade siete
regresiones funcionales/44 sin ejecutar. Parent lee el informe completo
`.codex-loot-residual-report.md` y los 64 contratos individuales; los body diffs
de las 87 adaptaciones quedan leídos en los siete `part-*.diff`, incluida
preparación, inputs, llamadas y observaciones, no sólo recuentos. Parent lee
además las fachadas de lifecycle, metadata/store item y sus observers; la fixture
de cast conserva su preparación anterior completa. Los valores esperados de
random properties conservan 9001/0 y -7001/123 a través del wrapper opaco; geometry
conserva ContactDistance 0.5. Empty encounter catalog sólo sostiene los casos
previos de encuentro sin metadata; no prueba jugadores bloqueados por un lock
real, exigido en la familia residual asignada. Equivalencia de todas las fachadas,
gates y montaje/aceptación aún pendiente. Parent lee los siete
Money negatives/44 assertions y sus forwards de apertura/consumo: normales sin
Loot authority, owner retirado/deactivado, ausencia de snapshot de Player,
generación stale y aplicación positiva al Player residente. Son casos escritos,
sin ejecución. Comparación independiente de los seis children físicos contra
PRE íntegro confirma 37 firmas y 37 cuerpos literales; informe source-only
`/tmp/sol-loot-physical-37-source-comparison-20260930.json`. No prueba imports,
montajes, visibilidad efectiva, compilación ni aceptación.
En el snapshot concurrente posterior, el ledger de destinos anterior tiene
58 rutas sin su cuerpo nominal: la familia está en traslado, no son 58 tests
demostrados ausentes. Navegación independiente de las firmas actuales localiza
los 151 nombres una sola vez, sin ausentes; informe
`/tmp/sol-loot-151-current-name-navigation-20260930.json`. El primer informe por
ruta `/tmp/sol-loot-151-body-source-comparison-20260930.json` conserva ocho cuerpos
literales en sus destinos anteriores; sus 58 misses no prueban pérdida ni
equivalencia de nuevos destinos. Reconciliar cuerpos/mounts al terminar el worker.
Referencia reconciliada declarada 266/1 372, sin incluir casos nuevos ni reemplazos.
La preparación completa64 termina y se asigna implementación de 62/316 con
ports/futuros reales para storage/COMMIT, observadores de callbacks, encounter
real con InstanceLockMgr/full GUID, policy GO completa opt-in, autoridad
local/cast y facades Quest existentes. Dos originales/7 assertions quedan
intactos y pendientes: direct claim exige exactamente Removed→Push y batch
disenchant Push→Push→Removed en el antiguo rail cfg(test); producción también
publica objetos/valores. Se añaden dos casos APP con secuencia productiva
completa, sin filtrar opcodes ni sustituir igualdad global por subsecuencia;
no acreditan retirada de los dos originales. El handoff antiguo de Quest no
bloquea. No ensanchar cfg(test), sanar owner observado, sustituir grant por
stub ni crear otro mirror. Mounts Session compartidos requieren turno entre
workers; no validación durante la barrida.

**Loot62 entregado, revisión e integración aún abiertas:** Zeno entrega los
62 originales/316 macros APP externos, dos casos adicionales de wire completo
y locators actualizados. En esa entrega declara referencia266 única y
residual151 con 149 externos/810 macros y dos privados históricos/7.
Parent lee ambos casos wire completos y los dos históricos, los ports/futuros
reales de storage, los tres escenarios encounter y su preparación mediante
InstanceLockMgr/full GUID/reset futuro. Lee también el diff completo de los
cores de aplicación (678 líneas), policy transitoria, fixtures de cast y sus
observaciones de los owners reales. Tres familias de diff leídas: encounter,
cast y lifecycle; las otras ocho adaptaciones siguen pendientes de revisión.
El snippet de imports/field/default de criteria espera el fin del turno
exclusivo de Banach en state/construction. Cast/Aura queda libre para Gossip.
Parent asigna sustituir los dos oracles históricos incompletos por los casos
productivos completos conservando sus nombres: qty1/2 y toda la cola, sin
filtros, incluyendo Create/Values y commit/reclaim de batch. Registrar esa
revisión explícita de oracle (2→4 y 5→7 macros, más asserts del parser),
separando baseline y current; no declararlos literales ni contarlos como dos
nuevos además de originales. Retirada y reconciliación de esa unidad pendientes;
no hay compilación, aceptación ni prueba real de durabilidad DB.
La revisión de los dos oracles queda implementada después: Zeno conserva sus
nombres históricos únicos en storage_wire, retira ambos archivos/mounts privados
y registra baseline266/1 372 frente a current266/1 376; residual151 pasa de
817 a 821 macros actuales y todos sus destinos son externos. Parent lee el diff
exacto y el lineage/operands; no son cuerpos literales ni dos nuevos adicionales.
Los otros 264 cuerpos quedan declarados sin cambio en esa revisión bounded.
Zeno prepara el consumidor CreatureLoot para el retiro del owner legado, sin
activar ni editar shared roots; falta revisar ocho familias de adaptación62.

**PlayerItems físico entregado parcialmente:** doce hotspots partidos, según
PRE íntegro `.agent-artifacts/player-items-physical-pre-13`; parent compara las
288 definiciones: cuerpos literales, trece ajustes de visibilidad conservando el
acceso anterior. Lee también los cuatro source guards adaptados. Compra tenía
una función de 938 líneas; parent lee la operación completa y
fija currency / item purchase / publication privados. COMMIT, aplicación runtime
síncrona y liberación money guard quedan en item purchase; publicación conserva
quest drain, currency losses, creates/counts y fences instance→realm→instance.
Parent lee los cuatro archivos actuales y compara directamente contra PRE:
currency literal, item hasta drop del money guard literal salvo retornos Option,
publicación literal. Raíz 178 y children 244/553/131; el bundle privado contiene
sólo los resultados movidos de esa compra, sin clone/context nuevo. La partición
no cambia ownership ni reduce por sí misma el total de World.

**Familia completa PlayerItems/apariencias asignada tras compra:** parent lee
raíz/siete children, owner `PlayerCollectionStateLikeCpp`, los 26 originales y
consumidores de login/teleport, account load/save, handlers favoritos y spell
AddSet. Contrasta `CollectionMgr.cpp:461–950`, `Player.cpp:11069–11125/18747–18804`
en `a5f8da2e`. Doce originales/51 assertions pasan a Domain (Has, cinco admisiones,
load/save appearances/illusions y dos completitud de sets); catorce APP/67 pasan
al target de integración. Los dos handlers Favorite y dos casos de puerto/fallo/
orden de cuenta siguen APP; sus tests no prueban ausencia de owner si adaptan
construcción con uno válido. Capturar PRE íntegro antes de la nueva migración.

El owner actual recibe admisión completa, favoritos, Has/providers y proyecciones
de load/save/sets, con módulos privados; sin otro CollectionState ni dependencia
normal Entities→Data. Un puerto específico de filas/facts y lecturas Player
mantiene el orden lazy de los 241 renglones actuales. Colección/permanente se
consulta al final, temporal no bloquea; calidad baja exige ambos flags. CanUseItem
parcial, holiday/artifact specialization ausentes y ItemSpecStats fallback siguen
gaps explícitos. No convertirlos en reparación dentro del refactor.

APP mantiene writers separados y sus fallos: permanente lee temporal, escribe
flags/conditional/values en Player, sólo si añadió flag escribe Collections y
después criteria. Temporal muta Collections antes del writer Player; no agrupar
closures ni sanar el fallo parcial. AddSet no llama CanAdd, mantiene ID cero,
duplicados, skips y último update. Set completion preserva slots -1/0/1,
primer slot completado y resultado true para set sin piezas resolubles.
Favorites mantienen New/Removed/Unchanged y el reapply/unapply actual, incluido
republish al unapply de Removed. Opcode 0xBADD conserva state sin packet.

Load preserva filtros de cero, orden BTree/dense y favoritos Unchanged; flags
Player preceden install Collections. Active-create prefiere collection blocks,
luego canonical Some incluso vacío, y sólo None usa proyección por IDs. Save
opera sobre el snapshot clonado, settle favorites antes de persistence y conserva
replace ignorado/plan Some y retención para retry/finalization actuales. Illusions
conservan defaults 3/13/22/23/34/43/44 y los bloques no vacíos, sin nuevo clamp.
Outfits pertenecen a EquipmentSets y su load se conserva literal; no llevarlos
a CollectionState. Criterios continúan representados en tests; no añadir motor.

Los catorce APP originales conservan nombres, inputs, assertions y packets/fallos;
fachadas feature-only expresan operaciones/observaciones completas. Fixture
ownerless, si imprescindible, requiere opt-in explícito default false; Some
handle, incluso stale, nunca permite fallback. No crear mirror mutable ni fixture
siempre válida que sustituya pruebas negativas. Escribir límites default/no GUID/
stale/prioridad canónica/fallos parciales; no ejecutar. Archivos producción/tests/
fixtures ≤600 naturales, DTOs únicos con aliases de compatibilidad cuando proceda.

**Appearance entregada, revisión/aceptación pendientes:** Pauli entrega 53 rutas
(19 modificadas, 29 nuevas, cinco shells retirados), owner anterior con children
privados y una fachada World de 72 líneas. Declara 26 originales únicos: 12 de
dominio/51 assertions y 14 externos/67; cuatro APP adicionales/15 assertions
permanecen internos cfg(test), no contarlos externos. Escribe 14 casos nuevos/65
sin ejecutarlos. PRE íntegro y mapas/diffs en `.agent-artifacts/appearance-family-pre/`;
Parent compara los 30 métodos anteriores de CollectionState literalmente con
el PRE íntegro. Confirma 30 nombres originales en sus destinos, incluidos los
cuatro internos; sólo esos cuatro cuerpos siguen literales. Los otros 26
requieren revisar sus adaptaciones, no inferir equivalencia de sus recuentos.
Lee los 14 casos nuevos completos y sus fixtures/default gates; no se ejecutan.
Doce funciones anteriores de CollectionAdapter siguen literales; sus dos métodos
is_empty se conservan literalmente en los DTO del owner. Informe independiente en
`/tmp/sol-appearance-original-source-comparison-20260930.json`. Parent lee los
26 originales adaptados completos y sus diffs/fixtures: doce Domain usan Player
y los catálogos reales; los catorce APP adoptan sólo el Player ya registrado y
restauran su identidad original. Los negativos de owners son casos adicionales.
Compara las 133 macros originales/current de los treinta casos: diez secuencias
de assertions siguen literales en tokens; las veinte restantes coinciden tras
proyectar únicamente los accesos owner/fachada revisados, con mensajes intactos.
La equivalencia de esos accesos se contrasta con los helpers y owners, no se
deduce del recuento. Los dos builders APP de rows Item y el loader de outfits
EquipmentSets conservan cuerpos literales frente al PRE. Informe de operands
en `/tmp/sol-appearance-assertion-operands-20260930.json`; el extractor concreto
resuelve las firmas con semicolons de arrays que el navegador genérico omite.
Admisión/fixtures, catálogo/publicación y loaders se revisan en fuente; no hay
ejecución ni cierre de presupuesto global o aceptación de la familia.
Los roots state/construction quedan libres por ACK de Pauli y Einstein, pendiente
turno con Loot. Pauli termina la preparación de B3 Gossip/Trainer y recibe la
implementación completa: 23 APP originales/111 macros más tres macros de helpers,
con PRE íntegro, helpers/mounts y consumers en
`.agent-artifacts/character-gossip-trainer-pre/`. Parent lee gossip completo,
trainer helpers, NPC/taxi/feign/Aura gates y las funciones Gossip/Select/Binder
del C++ a5f8da2e. Todos los escenarios permanecen APP externos: no acredita pureza
de dominio. Binder, conditions, catalog_menu y selection pasan a hijos privados.
El opt-in Gossip es privado/defaultfalse; la reconstrucción NPC/taxi/Aura debe
usar los campos/algoritmos previos sólo sin handle, nunca curar un Some rechazado.
Feign conserva DIED lookup→snapshot→slots→remove_aura→clear separado. Shared
roots/Aura se integran mediante snippets y turno con Character/Loot, sin habilitar
automáticamente el fixture Lifecycle ni cambiar registry/gaps. Retirada de los 23
montajes y revisión completa pendientes, sin validación.

Pauli entrega la implementación Gossip23 con PRE/diff íntegros y patch compartido
`.codex/character-gossip-complete-integration.patch`: los cinco originales siguen
montados hasta integrar/retirar en turno parental. Declara 111 macros originales
más tres de helpers conservadas y seis negativos nuevos/29 macros escritos.
Parent lee el patch, los cambios de NPC/taxi/Aura/vitals/phase, facade de 178 líneas
y cinco negativos de fixture_modes; la comparación independiente de los 23 casos
y helpers sigue pendiente. El predicate mantiene cfg(test) histórico implícito,
pero requiere opt-in y GUID sólo en consumidores feature sin handle; Some nunca
permite reconstrucción. Pauli recibe el corte privado por responsabilidades de
taxi/operations.rs (818), conservando cuerpos y consumers, sin validación.

**Gossip23 integrado en fuente:** parent aplica el patch compartido completo y
retira las cinco suites privadas. Contraste independiente de los seis métodos
productivos trasladados: fn/signature/body byte-idénticos. Los 23 casos conservan
111 macros por caso y sus operands tras sólo los forwards explícitos; los
mensajes son literales. Informes `/tmp/sol-gossip-6-production-functions-source-comparison-20260930.json`
y `/tmp/sol-gossip-23-assertion-operands-source-comparison-20260930.json`.
Parent lee también el sexto negativo de catálogo stale y los cinco de modos,
29 macros nuevos escritos. La revisión completa de arrange/helpers todavía no
se sustituye por estos contrastes; ninguna ejecución ni aceptación.

**Taxi, corte físico integrado:** operations 818 pasa a root12 y cinco hijos
privados de 256/222/94/155/112 líneas (vehicle/admission/protocol/flight/transport).
Parent contrasta independientemente las 50 funciones y todos los fragments con
PRE íntegro: firmas/cuerpos y los raw ranges con cfg/comments/visibility son
literales. Aplica sólo los dos locators de política al child admission_routes;
no regenera inventario/baseline. `/tmp/sol-taxi-50-source-comparison-20260930.json`
es comparación de fuente, no aceptación ni reducción del logical owner Session.
Pauli prepara ahora una familia cohesiva de subestado operativo Session y su
construcción/consumidores, con roots reservados hasta fijar el contrato.

**Subestados Session, contrato fijado para implementación:** tras la preparación
acotada y lectura parental de los 14 tipos actuales, Pauli recibe turno exclusivo
de state/construction. Traslada las definiciones completas sin cambiar nombres,
cfg, visibilidad o campos a cinco hijos privados: conexión/admisión/cuenta/realm,
coordinación runtime, spell/quest, lifecycle y social. Son subestados ya existentes;
la división física no acredita retirada de gameplay del logical owner.
Agrupa además sólo cinco campos contiguos de publicación (farsight observado y
cuatro sets de deduplicación) en un subestado privado en el mismo sitio y orden
de fields/defaults. Cutoff, fixture de visibilidad, fase GO, última posición y
client-visible aliases permanecen fuera para preservar intercalación/drop.
Actualiza todos los accesos de esos cinco campos, sin nuevas API públicas,
espejos, locks o getters por escenario. Session-root imports/policy quedan como
snippets parent. State1811/construction984 son el PRE de esta unidad: no promete
30% ni cierre del presupuesto por la mera agrupación; la reducción restante
requiere retirar familias completas de catálogos/configuración. Sin validación.

**Subestados Session entregados e integrados, sin aceptación:** state.rs queda
en 1 532 líneas y construction/world_session.rs en 987. Parent compara las 14
definiciones completas (una ocurrencia literal cada una), los once consumidores
como archivos completos tras quitar sólo el prefijo nuevo, la declaración entera
WorldSession al expandir el grupo y el constructor entero al deshacer defaults/import.
Integra los dos locators SessionDirectory; session/mod.rs no cambia. Comparación
independiente de fuente en `/tmp/sol-session-private-state-source-comparison-20260930.json`.
Esto no acredita aceptación ni retiro del logical owner; ambos budgets siguen abiertos.

**B3 CreatureMelee, integración fuente:** parent integra los cinco mounts de la
entrega y compara independientemente los 48 cuerpos originales/535 assertions:
idénticos después de los aliases explícitos de fixture, sin cambiar expectativas
ni mensajes. 47 casos APP y uno Domain; 21 archivos privados retirados por la
entrega. Comparación en `/tmp/sol-creature-melee-48-source-comparison-20260930.json`.
No se ejecutaron pruebas; helpers/cfg y composición necesitan aceptación final.

Recuento fuente concurrente del 2026-09-30 12:44 UTC: wow-world src 356 357 y
tests 54 381, total 410 738 líneas Rust. No es aceptación ni una reducción frente
al recuento anterior: los originales Gossip aún coexistían con sus destinos.
Mover suites dentro de wow-world no reduce su masa total; el límite 200 000 y
la retirada del doble owner/productor de Creature permanecen abiertos.

Recuento fuente concurrente del 2026-09-30 14:02 UTC: wow-world src 347 752 y
tests 61 150, total 408 902 líneas Rust: 1 836 menos que el corte 12:44.
Es masa del checkout en implementación, no evidencia verde ni cierre del límite.
CreatureLoot entrega motor y adapters (raíces World 753→472 y 643→260), 14 nuevos
casos/55 assertions escritos; kill entrega motor compartido y entrada selected
dormante, World −34 y 15 casos/81 assertions escritos. Parent integra los cuatro
mounts Loot y los dos Kill, lee motores/adapters/casos nuevos y los diffs de
consumidores. Confirma independientemente 23 cuerpos Loot literales y cinco
archivos originales Kill byte-idénticos; APP fuera de sus dos closures sustituidas
es literal. Evidencia de fuente en `/tmp/sol-creature-loot-23-source-comparison-20260930.json`
y `/tmp/sol-creature-kill-source-comparison-20260930.json`. El pipeline único, retiro de productores y de
rebind siguen abiertos; no se activan paths preparados por la mera entrega.

**Continuación 2026-09-30 15:01 UTC, barrida sin validación:** parent integra los
dos mounts Unit visibility y los tres children Collection. Unit conserva el
archivo completo fuera de sus tres delegados y sus 24 tests originales literales;
facts/kernel315 y diecisiete casos nuevos/294 líneas revisados en fuente. Su
adapter ops695 sigue por encima de 600; esta entrega no cierra el presupuesto.
Colecciones transfiere preparación/proyección/mejoras a Entities (172 líneas de
reglas, 271 de tests nuevos), con reducción real de 67 líneas World. Parent lee
el diff completo y confirma independientemente siete archivos originales de
tests, catorce funciones de caso y 95 funciones restantes literales. Comparaciones
en `/tmp/sol-unit-visibility-source-comparison-20260930.json` y
`/tmp/sol-account-collection-complete-source-comparison-20260930.json`.

Admisión compartida loaded-grid107 y diez casos nuevos revisados y montados:
los cuatro archivos consumidores son literales fuera de sus respectivos bloques
sustituidos, y los tres archivos/15 casos prepared previos siguen byte-idénticos.
Prueba fuente independiente en `/tmp/sol-shared-loaded-grid-source-comparison-20260930.json`.
Timers/planners/counters y firmas Record siguen en su punto anterior; no se activa
la factory Actor.

**Integración posterior 2026-09-30 16:30 UTC, fuente sin validar:** los tres
mounts Group y el getter privado reserved Loot ya están integrados. Parent lee
los doce casos Group, sus once helpers y tres casos de wiring; confirma los
cuerpos completos tras los aliases revisados (literales de strings conservados),
diecisiete funciones mixtas restantes y los dos decodificadores literales en
`/tmp/sol-group-session-12-source-comparison-20260930.json`. Reserved108 y diez
casos/50 assertions revisados completos; cuatro inputs anteriores byte-idénticos
y manager literal fuera de mount/provenance en
`/tmp/sol-reserved-loot-source-comparison-20260930.json`.

Los seis mounts Kill están integrados: cinco casos completos revisados, sus 27
assertions conservan operands/messages/orden tras aliases; prehook/Pet y dos
constructores son literales retirando sólo el guard opt-in; post-success conserva
la secuencia entera. Nueve funciones Kill y 25 casos mixtos restantes son
literales, con comparación independiente en
`/tmp/sol-kill-app-external-source-comparison-20260930.json`. La corrección del
input Pet handleless con feature está revisada e integrada: tres ramas opt-in y
seis gates de disponibilidad en dos roots, sin modificar cfg(test), habilitar
fallback normal ni reparar Some(stale). Parent lee los tres casos nuevos/25
assertions; los cinco originales permanecen byte-idénticos, así como el prefix
con helpers/tres casos de wiring previos. Los archivos completos Pet/mode y los
dos roots son literales al retirar sólo esas ramas/gates, respectivamente, en
`/tmp/sol-kill-pet-input-source-comparison-20260930.json`. Pet613 sigue pendiente
del presupuesto físico; ninguna prueba se ha ejecutado.

Toys transfiere tres originales/11 assertions y dos transiciones/proyección a
Entities, retira 52 líneas World (9 de adapters y 43 de tests) y parent integra
mount/retirada de cuatro operaciones canónicas literales. Once APP originales y
93 funciones ajenas permanecen literales en
`/tmp/sol-toy-transitions-source-comparison-20260930.json`; tres casos nuevos/14
assertions escritos y revisados, ninguno ejecutado.

Map/Legacy visibility facts378 y diecisiete casos/359 líneas están revisados y
montados; una enumeración Legacy común conserva su cuerpo al restaurar sólo
capture→clone, y el archivo entero fuera del bloque extraído es literal. Los
cuatro originales World/14 assertions y Unit facts previos siguen intactos en
`/tmp/sol-map-creature-visibility-source-comparison-20260930.json`. La adaptación
de los dos loops World y sus queries/publicación está implementada y revisada:
misma ventana, precedencia Legacy, spline canónico None y Player/CanSee posterior
al guard. Parent lee el diff717 y cuatro casos APP nuevos; confirma archivos
query/handler literales fuera de dos funciones cada uno, los trece casos/archivo
World literal tras dos aliases de getters, seis broadcast y dos tests inline
originales literales. Flags/points y payload Aura conservan sus cuerpos completos
en `/tmp/sol-world-creature-facts-consumers-source-comparison-20260930.json`.
CREATE y auras comparten la captura inmutable; no se clona Creature/MotionMaster
en estos lectores. El split físico está revisado e integrado: handler978→root89
con children185/440/318, escenarios891→root10 con children427/364/105. Parent
reconstruye el refresh completo desde children reales y facade y obtiene fuente
literal tras sólo aliases de argumentos/return/import. Nearby, viewer y dos
wrappers cfg(test), y los trece cuerpos originales, siguen literales en
`/tmp/sol-visibility-physical-source-comparison-20260930.json`. No retiro adicional
de owner por este split ni aceptación ejecutada; los roots reales son byte-idénticos
a los candidates revisados.

Recuento concurrente 16:41 UTC: World src346472/tests62898, total409370. Incluye
tests privados y entregas aún pendientes de integración; no es aceptación ni
cierre del presupuesto200000. No se presentan
las capturas preparadas ni los traslados internos de suites como retiro de owner.

Group/Conditions owned está revisado e integrado con sus cuatro exports en dos
roots: core266/routing127/shared112, doce casos nuevos y oracle201 leídos completos.
Parent confirma firmas públicas, archivo Group fuera de dos métodos/mount,
motor shared y prefix de gates literales (sólo dos returns envueltos en DTO);
el match/counters/push Record sigue literal en su punto original. Dos funciones
oracle completas son literales tras aliases de nombre, y trece inputs anteriores,
incluidos los quince casos prepared y diez shared-admission, quedan byte-idénticos
en `/tmp/sol-group-owned-loader-source-comparison-20260930.json`. Los resultados
Actor conservan incoming completo; sus duplicados bloquean Add bajo el contrato
dormante aprobado, sin afirmar equivalencia con el refresh Record ni activarlo.

Reset seasonal está revisado e integrado en cuatro roots: Domain134 consume el
mismo snapshot y APP458 conserva fence, writeback ignorado antes de metadata,
clones Arc por ID y bit writer. Parent lee seis originales Domain218, cuatro
stages85, cuatro originales admission y tres bridges APP131; los diez nombres/44
assertions conservan operands/messages/orden tras mappings explícitos. Cuarenta
funciones mixtas restantes, prefix de eligibility y archivo giver fuera de una
función son literales en `/tmp/sol-seasonal-quest-source-comparison-20260930.json`.
No se afirma cuerpo literal de los casos adaptados ni interleaving fallido probado.

DependentPrevious está revisado e integrado con un mount: Domain52 tiene todo el
algoritmo y callbacks lazy de catálogo, World sólo delega. Un original puro/cuatro
macros conserva sus operands tras alias; ocho casos Domain nuevos y bridge APP
real están escritos. Parent confirma ambos World fuera de una función/caso y
seis inputs/callers completos literales en
`/tmp/sol-dependent-previous-quest-source-comparison-20260930.json`. Los cinco APP
reales permanecen en APP; CPP ASSERT versus Rust fail-closed y HashMap versus
índice exclusivo siguen como gaps existentes, sin reparación.

Los diez literales Item activos en Character están revisados e integrados con un
mount de tres líneas: builder57 totalmente explícito, sin Default. Parent expande
arguments/overrides de POST y confirma 175 fields y 33 funciones completas tras
sólo esa expansión (28 literales), fixtures_3 intacto, en
`/tmp/sol-item-row-fixtures-source-comparison-20260930.json`. Reducción neta World79;
no se presenta como retiro de gameplay. Los treinta literales Session también están
revisados: parent lee diff837, el builder explícito y los overrides, expande los
argumentos POST reales y confirma los cuatro archivos completos equivalentes,
510 valores de campos/30 instancias y prefix del fixture literal en
`/tmp/sol-session-item30-source-comparison-20260930.json`. Reducción neta World352;
los dos archivos mixtos657/804 aún requieren división física. Se asigna consolidar
cuatro constructores QuestTemplate/68 campos explícitos, conservando cuatro helpers
y las variantes type/title; las longitudes de arrays son valores equivalentes de
los mismos constants, no identidad de tokens. Esa segunda entrega está revisada
e integrada en los dos roots: diff401 y constructor78, cuatro archivos completos
fuera de helper literales y 272 valores POST equivalentes en
`/tmp/sol-quest-template-fixture-source-comparison-20260930.json`. Conserva la
feature del facade y módulo privado cfg(test/feature), sin Default ni dependencia
nueva. Reducción neta World195 después de las cinco líneas de integración.

Otros cinco constructores QuestTemplate están implementados y revisados en fuente:
eligibility471→402, projection187→118, remote-loot206→137, handler-support621→552
y Talent575→506. Parent lee diff400 completo, expande el builder actual y los
arguments POST y confirma 340 valores equivalentes y cinco archivos completos
fuera del cuerpo del helper literales en
`/tmp/sol-quest-template-five-additional-source-comparison-20260930.json`.
Reducción neta World345; no nuevas API, mounts, dependencias ni cambios de gates.
Sin compilación ni aceptación.

Otros 18 literales Item Session (12 Record/seis Sparse) están consolidados y
revisados: diff482 completo, cuatro builders anteriores íntegros, 270 valores
conservados. Parent confirma los cuatro archivos completos tras únicamente las
18 sustituciones de expresión revisadas y cinco consumidores adicionales
literales en `/tmp/sol-session-item18-post-source-comparison-20260930.json`.
Items1 queda857 y fixture274; reducción neta World192 antes de sus splits.
Scaling77/0x200, precios, arrays, clases, durabilidad, slots de bolsas y orden
de instalación mantienen valores explícitos. Los tres archivos mixtos siguen
fuera de presupuesto en esa entrega. Items7/8 ya están físicamente integrados:
Items8 raíz9/children205 y515; parent reconstruye las723 líneas originales y
cuatro funciones literalmente, ocho inputs completos intactos en
`/tmp/sol-items8-physical-source-comparison-20260930.json`.
Items7 raíz18/children117,246,150,246; parent reconstruye las758 líneas originales,
siete funciones/38 macros y seis inputs completos literales en
`/tmp/sol-items7-physical-source-comparison-20260930.json`.
Ambos son organización física, sin reducción de responsabilidades o gameplay.

El cierre final del tick está implementado y revisado: try_finalize devuelve la
continuación original en preflight y try_complete conserva work/summary; Ok(None)
es finalizado, sin replay. Parent lee once casos nuevos/62 macros por caso y siete
compartidas, conserva el suffix completo de efectos, trece métodos manager,
cuatro APP y siete inputs previos literales en
`/tmp/sol-final-object-tick-source-comparison-20260930.json`. Los wrappers anteriores
conservan descarte/Option. La composición tipada del consumidor está implementada
y revisada: seis paths, ocho casos nuevos/64 macros escritos, Failure y la misma
TickAdmission retenidos antes de soltar guards. El guard tras el interval impide
otro clock/admission/pass mientras existe fallo; no retry automático. Ok(None)
continúa PostTail y FullyFinished, reparando el None→break introducido en este WIP
que HEAD no tenía. Parent lee diff672 completo y confirma ocho inputs anteriores,
diez funciones completas, closure GameObject y todo el suffix de publicación,
PostTail/shutdown literales en
`/tmp/sol-object-consumer-source-comparison-20260930.json`. La prueba escrita del
productor vacío usa su spawn real; el fallo retenido prueba work/ledger reales,
sin afirmar inyección de un fallo inalcanzable en el productor. Begin recuperable
está implementado y revisado: diff624 completo, diez casos nuevos/105 assertions
escritos, BeforePrefix conserva plan y AfterPrefix error/plan/summary originales;
retry devuelve el fallo íntegro sin replay. Parent confirma trece inputs completos,
catorce funciones Manager, seis APP y siete casos del consumidor literales, el
suffix BEGIN de éxito completo y el prefix completo con orden efectivo original
en `/tmp/sol-object-begin-source-comparison-20260930.json`.
AfterPrefix se prueba como empaquetado de frontera: no se afirma alcanzable bajo
el flujo normal de préstamo exclusivo. Abort transfer y settlement siguen abiertos.
Nada ejecutado ni quiescencia atribuida a abort.

Pool owned está revisado e integrado con dos exports parent: root571,
materialized282, routing216 y despawn139; ocho casos nuevos en children≤247.
Parent lee diff1887 completo y confirma cuatro motores recursivos completos tras
aliases/receipt args, cinco operaciones despawn, seis oracles, trece funciones
restantes y 23 inputs completos literales en
`/tmp/sol-pool-owned-source-comparison-20260930.json`. NoLoader sigue siendo la
ausencia real de callback; Unavailable y PreparationRejected conservan contratos
distintos. Un solo planner, mismo orden Pool/GO/Creature, despawn antes de RespawnOne,
match/counters/push Record en su sitio; incoming Actor permanece íntegro.
Catalog está implementado, revisado e integrado con dos exports parent: raíz338,
routing270 y doce casos nuevos escritos en children≤191. Parent lee diff1154
completo y confirma 36 inputs completos, dos wrappers y nueve funciones routing
literales, oracle original íntegro, cola íntegra fuera de los tres bloques de
receipts revisados y match anterior Pool compartido íntegro salvo metadatos de
plan en `/tmp/sol-catalog-owned-source-comparison-20260930.json`.
Usa el mismo core/sink, conservando timers después de acciones
pooled y antes de admisión no pooled, loader antes de delete y políticas de fallo.
Los callers productivos/factory/mirrors siguen Record, sin activar owned.

Unit ops_1 también está revisado e integrado: root695→12, children de presencia175,
estado de visibilidad140, revisions88 y control de combate307. Parent reconstruye
el archivo original completo desde los children actuales: 76 métodos/cuerpos y
docs literales al restaurar sólo dos spellings de visibilidad efectiva equivalente;
cinco otros inputs completos/57 tests intactos en
`/tmp/sol-unit-ops1-physical-source-comparison-20260930.json`. Sin nuevo owner ni
reducción World atribuida a esta división.

Los seis workers continúan con cortes físicos Items7/8, unificación signed-Previous
Quest, retorno propietario por stop cooperativo, retirada del par Creature/Record
clonado del factory productivo y contrato de driver del productor original Kill.
Los ocho tests puros cooldown ya son Domain; sus
nueve APP quedan allí. La decisión daily/DF duplicada de sharing ya está retirada
y revisada; Domain30/tests71, mismo quest_id en callbacks lazy y prioridad DF.
Cuatro casos lazy y un bridge registrado/mailbox están escritos; parent lee el
diff completo y confirma diecisiete funciones originales, catorce otros inputs,
sharing entero fuera de la decisión y prefix íntegro quest_4 literales en
`/tmp/sol-quest-periodic-cooldown-source-comparison-20260930.json`. Mount/delegate
del root Domain integrados; Week/Month/recurrence y gates/publicación conservados,
sin inventar masa pura retirada. Quest_4 crece842→887; su split físico ya está
integrado: raíz9, pool-admission102, receiver-admission312 y requirements495.
Parent compara independientemente las 21 funciones completas y sus atributos
tokio, sin duplicados ni cambios de nombres/cuerpos, y aplica el patch del worker;
`/tmp/sol-quest-sharing-physical-source-comparison-20260930.json` conserva esa
evidencia estática. El split no retira responsabilidades ni reduce el total World.
El melee actual no tiene slot que continuar: no se
fabrica identidad desde GUID ni se reutiliza el slot Spell de otro primario.
La alternativa B se selecciona para diseñar el productor: primer killed real bajo
el guard original, root del atacante seleccionado y targets realmente dañados
sin ampliar el workset de update. Tras leer completos engine/commit/split/share,
gates y tipos actuales, collector/fuente/primer slot están implementados, revisados
e integrados exclusivamente en Map: entry100, collector165 y cinco archivos del
motor≤417. Parent lee los cuerpos nuevos, cinco diffs completos y los 14 casos
nuevos/113 assertions escritos, y compara independientemente catorce funciones
originales completas tras retirar sólo threading/capture y variables del mismo
lookup; cinco cuerpos incluyendo la entrada anterior son literales, cuatro
inputs previos íntegros en
`/tmp/sol-original-melee-kill-capture-source-comparison-20260930.json`.
Integra el único reexport crate-private. Captura Split→Share→Primary sólo tras
killed real; primera muerte reserva una ROOT del atacante seleccionado, target
real puede estar fuera del update workset, fuente/authority/revisiones conservadas
bajo el guard original. Sin alive gate tardío, dedup ni retry de slot rechazado;
Record conserva NoActor y sus efectos. Batch retiene token original y outcome
parcial completo; source es postcommit, no posthooks. No consumer completo
Kill ni activación. Instalar/finalizar/settle/dispose esperan su contrato de driver.
La regla completa signed-Previous está implementada, revisada e integrada en
Player: Domain31/tests86, Worldrequirements407 y APPprerequisites433. Parent lee
diff222 completo y confirma World entero fuera de comentario corregido/decisión,
prefix APP íntegro con ocho funciones originales y diez inputs completos literales
en `/tmp/sol-quest-previous-policy-source-comparison-20260930.json`.
Cuatro casos lazy y un bridge registrado COMPLETE/FAILED escritos, sin ejecutar;
mount/delegación parent integrados. Conserva cero/signo/INCOMPLETE/unsigned_abs y
no mueve tests de mailbox/paquetes a Domain ni cambia gates posteriores.
El driver
reserved generation-origin está implementado y revisado; los roots
compartidos quedan parent; ningún
worker ejecuta Cargo/fmt/metadata/tests/QA ni activa productores Actor. Los contratos
concretos son los PRE/propuestas de cada familia, no nuevas macros.

Recuento concurrente posterior 17:15 UTC: World src346415/tests62805,
total409220; incluye trabajo escrito, no aceptación ni attribution de producción.
La consolidación baja líneas y las pruebas nuevas pueden aumentarlas; un split
interno no acredita reducción. Disco observado17:06: 38GiB libres/193G81%,
sin nuevas compilaciones ni limpieza y con la caché activa preservada.

Recuento concurrente posterior a integrar Quest_4, 17:32 UTC: World src346208,
tests62698, total408906. Incluye código y tests; no es una cifra de producción
ni aceptación. Catalog owned y BEGIN recuperable ya tienen revisión parental de
diff completo y comparación independiente de fuente; sus casos no están ejecutados.
Pet ya está revisado e integrado: raíz613→11; canonical236, lifecycle102,
action-bar50, kill-fixture34 y movement151; petition-trace55 bajo Session
social_requests existente, cuyos tipos y fields mantienen su owner. Parent lee
los seis children completos y reconstruye las 613 líneas originales literalmente
restaurando sólo cinco spellings de visibilidad efectiva; 36 inputs previos
íntegros en `/tmp/sol-pet-physical-source-comparison-20260930.json`.
Conserva las tres ramas feature y la excepción stale existente sólo cfg(test).
Quest_5 ya está integrado: raíz862→9, prerequisites374, gates307 y delivery200.
Parent confirma las 17 funciones completas, nombres únicos y atributos tokio
literales en `/tmp/sol-quest5-physical-source-comparison-20260930.json`.
No hay movimiento a Domain ni reducción global atribuida a estos splits.
Productor original Kill continúa en
implementación. Factory loaded-grid y disposición/shutdown de fallos Objects
están en preparación acotada para resolver sus fronteras pendientes.
Se mantiene la barrida completa antes de compilar/probar.

Continuación 18:05 UTC, implementaciones aún sin aceptación: el constructor
Session conserva 429 campos/987 líneas; extraer Admission a un helper de17 no
resuelve el presupuesto y no está aprobado. Su agrupación requiere responsabilidades
reales, no otro bag/macro/Default. Items8 tiene corte contratado en dos children
205/515 y Items7 en cuatro117/246/150/246, roots parent reservados.
PreviousQuest comparte su regla completa cero/signo/INCOMPLETE/unsigned_abs entre
Player y sharing, con callbacks lazy; cinco APP existentes permanecen APP.
El factory real retirará sólo el par Creature/Record clonado mediante un creador
único y MOVE en el consumidor productivo; el resolver compatible mantiene su DTO
actual. Settlement Option, CastGUID parciales y temporización constructor/mirror
permanecen abiertos para el siguiente contrato owned; no existe un algoritmo
World de factory pendiente de trasladar y no se atribuye retiro World al alias.
El stop cooperativo devolverá failure y la misma TickAdmission por el JoinHandle
existente, también cuando supervisión haya consumido su resultado; no equivale
a FullyFinished, disposición, quiescencia ni settlement. Mantiene gates y orden
de shutdown; abort/timeout/drop continúan sin receipt probado.
Recuento escrito concurrente18:05: src346757/tests62785, total409542; incluye
children nuevos todavía sin montar y sus donors originales, por lo que no es
medida de retiro integrado. Último recuento con los splits Quest/Pet integrados
antes de estos children: src346044/tests62726,total408770 (17:47).
Parent vuelve a leer Unit::Kill10457–10763 completo en a5f8da2e: Loot precede
KillRewarder y procs Kill/Death; AI/ScriptMgr van después. No usar "after hooks"
como un único punto previo a generación: invertiría ese orden. El batch actual
conserva hechos postcommit/before-hook; falta executor real y continuidad,
y las trazas opt-in no ejecutan hooks. El siguiente contrato Map de ledger
ROOT/TARGET permanece en preparación; no hay receipt ficticio de completion.
Sin Cargo/check/build/test/fmt/runner/QA. Disco observado17:40:38GiB libres;
caché activa preservada y sin compilaciones en marcha en esa observación.

Checkpoint local18:18 UTC, HEADd6c92488/584-map-manager-domain, sin mutación Git:
Items7/8 y signed-Previous montados; World src346062/tests62785, total408847.
Todos los recuentos incluyen tests/fixtures; fuente revisada no significa verde
ni aceptación. Continúa abierto el objetivo World≤200000/owner≤20000 y la macro.
Scopes siguientes: Items1 físico (Huygens), familia duplicada de fixtures restante
(Pauli), próxima regla Quest realmente equivalente (Banach), pending ledger
original ROOT/TARGET (Einstein), factory Creature move-only (Russell) y retorno
propietario de stop cooperativo (Zeno). El pending valida la primera ocurrencia
sin hooks/avance/disposición/cleanup; no ofrece completion artificial ni nuevo
slot. Sus errores deben conservar pending/token/outcome/batch originales.
En este checkpoint Factory CreatedCreature y stop cooperativo estaban pendientes;
su revisión e integración posteriores quedan registradas a continuación.

Checkpoint local18:38 UTC, mismo HEAD y sin mutación Git: World src346071,
tests62785, total408856. Items1 está montado: donor857→raíz10 y children
201/398/257; reconstrucción literal completa, quince funciones y seis inputs
conservados en `/tmp/sol-items1-physical-source-comparison-20260930.json`.
Es división física, no retiro de responsabilidad ni reducción global.
CreatedCreature está revisado e integrado: diff386 completo, seis casos nuevos
escritos, core privado131 y proyección Record que mueve la Creature original;
el resolver compatible conserva su DTO/clone. Parent compara algoritmo completo,
proyección, caller real y provenance, más21 inputs íntegros en
`/tmp/sol-created-creature-source-comparison-20260930.json`. Ambos mounts aplicados;
la raíz717 sigue excediendo el presupuesto y Option/efectos parciales se preservan.
Stop cooperativo está revisado e integrado: diff515, siete casos nuevos/55 macros
escritos y facade montada. El mismo task devuelve failure y TickAdmission originales;
supervisión conserva ese resultado y el rechazo de shutdown incompleto.
Cuatro archivos completos fuera de los cambios explícitos y quince inputs
literales en `/tmp/sol-cooperative-stop-source-comparison-20260930.json`.
Sin nueva quiescencia, disposición, cleanup ni bypass de gates.
Pending melee original ROOT/TARGET entregado; su revisión posterior queda abajo.
Los seis scopes continúan: familia real de estado Session/Player (Huygens,
preparación), once literals equippedItem (Pauli, implementación;140 campos
contrastados con builders actuales), Min/MaxLevel en QuestTemplate (Banach,
implementación), consumo Loot desde pending original (Einstein, preparación),
rechazo owned de provenance tras materialización (Russell, preparación) y
conservación de owners tras rechazo de quiescencia (Zeno, preparación).
Sin Cargo/check/build/test/fmt/metadata/runner/QA; ninguna entrega es aceptación.
Disco observado18:38:38GiB libres, caché activa preservada. El objetivo completo
sigue abierto: World≤200000/owner≤20000, retiro real B6 y aceptación terminal.

Continuación18:58, sin Git ni aceptación: parent termina la revisión completa
de Pending melee (raíz171, motor125 y tres children de pruebas), catorce casos/57
macros escritos; ocho inputs completos y raíz fuera de mount/conversión literales
en `/tmp/sol-melee-original-pending-source-comparison-20260930.json`.
ROOT valida antes de TARGET, ambos originales; outcome/token/batch/cursor0 se
conservan, sin hooks, nueva reserva, avance, disposición ni activación del driver.
Min/MaxLevel está integrado en QuestTemplate existente: owner14/seis casos106,
los dos consumidores World conservan el resto de sus archivos completos; cinco
casos APP/18 assertions/8 matches intactos y dos mounts parent aplicados.
La comparación independiente incluye raíces Model tras esos mounts en
`/tmp/sol-quest-level-policy-source-comparison-20260930.json`; World−8 líneas.
EquippedItem11 revisado: diff286 completo, 140 campos contrastados con los builders
actuales y seis archivos completos idénticos salvo las once sustituciones
en `/tmp/sol-equipped-item11-post-source-comparison-20260930.json`; World−93,
38 casos/215 macros preservados, incluyendo nueve afectados/69. Builder intacto.
Recuento18:53: src345970/tests62785,total408755, anterior a los nuevos cortes.
Los scopes activos pasan a unidades completas: Loot original→generación APP real→
instalación sobre TARGET (Einstein, implementación, sin nuevo slot ni driver);
fábrica común con rechazo propietario tras materialización y ambos preparadores
typed (Russell, implementación; Option/logs/efectos parciales compatibles);
abandono recuperable del plan y consumidor BeforePrefix (Zeno, implementación,
AfterPrefix retenido, éxito sólo MapIdle sin settlement del ticket);
familia amplia restante de requisitos Quest (Banach, preparación);
familia completa de literals de fixtures con builders existentes (Pauli,
preparación); y motor de estado representado de GameObject (Huygens, preparación).
Este último conserva el estado per-Session actual: no unifica ocultamente sus
cooldowns/defaults/chair slots con la autoridad canónica compartida.
El contraste completo de shutdown corrige la hipótesis anterior: fallo de
quiescencia marca ERROR y continúa hasta stop; no retorna antes. Su resultado
retenido se pierde al devolver sólo ExitCode, y falta settlement real de Objects.
No se implementa un outcome privado que termine igualmente descartando owners.
Continúa la barrida; no compilación/check/test/fmt/metadata/runner/QA ni publicación.

Continuación19:47 UTC, mismo HEADd6c92488 y sin aceptación: factory/provenance
move-only revisado e integrado; un motor privado222 sirve los tres builders y
ambos preparadores. Los rechazos conservan Records completos y efectos parciales
previos; no rollback/retry ni otra Creature. Raíz runtime/map942→791, aún sobre
presupuesto. Diez casos nuevos escritos y18 inputs completos conservados en
`/tmp/sol-owned-provenance-source-comparison-20260930.json`.
Abandono recuperable montado: Map conserva el plan rechazado y APP BeforePrefix
lo devuelve; AfterPrefix devuelve failure original intacto. Seis casos nuevos/127
macros escritos, diez inputs completos y dos raíces fuera de mounts literales en
`/tmp/sol-recoverable-abandon-source-comparison-20260930.json`.
El siguiente consumidor conserva los puntos terminales actuales: los dos rechazos
BeforeObjects hacen break, sin completar ticket. La implementación asignada
devolverá directamente plan/participants/TickAdmission/permisos originales en esos
mismos puntos; no introduce espera por stop, held cell, retry ni receipt.
Sharing acceptance completo montado en Player: motor63/doce casos236, World
prerequisites340→298. Conserva explícitamente la política parcial actual distinta
de CanTakeQuest local; cuatro inputs APP nuevos exponen esas diferencias. Tres
APP/seis Domain originales y veinte inputs completos conservados en
`/tmp/sol-quest-sharing-acceptance-source-comparison-20260930.json`; ningún test APP
se presenta como retirado hacia Domain.
Lote físico de cinco escenarios montado: raíces18/18/16/18/15 y dieciocho children
≤575,52 funciones completas/296 macros literales;14 inputs completos conservados
en `/tmp/sol-five-scenario-splits-projection-source-comparison-20260930.json`.
Esto añade85 líneas físicas y no retira responsabilidad World. Recuento concurrente
posterior19:45: src346173/tests63365,total409538; incluye implementación y pruebas
en curso, no retirada integrada estable ni producción atribuida. La reducción
global sigue insuficiente frente al objetivo200000/owner20000.
La siguiente coordinación prioriza entregas completas y retiro de caminos
duplicados: Loot original→generación APP→TARGET (Einstein), motor completo
GameObject prestado per-Session (Huygens), retorno terminal BeforeObjects (Zeno),
birth canónico en el punto de construcción real (Russell, diseño acotado), rail
Character86 completo (Banach) y revisión conjunta del resto de Loot62 (Pauli).
La revisión de timing demuestra que construir antes de Fresh no equivale al
mirror actual: snapshot Creature es pre-Add, wrapper se construye después. El
futuro owner es el canónico post-Add; evitar repetir Add/Motion/índices/loot o
adelantar RNG/waypoint. No se activa el nuevo backend ni se retira aún el productor
legado. CPP completo y propuesta en
`/tmp/rustycore-b6-loaded-grid-construction-timing-proposal.txt`.
Petición del usuario de reducir demora: agrupar revisiones y cortes físicos,
evitar nuevos ciclos de comparación/documentación por helper y priorizar cierre
de responsabilidades completas. Mantener los contratos ya fijados y la barrida
íntegra antes de compilar/probar; source comparisons no son aceptación.
Disco observado19:25:38GiB libres/193G81%, caché activa preservada. Ningún
Cargo/check/build/test/fmt/metadata/runner/QA/publicación en esta continuación.

Checkpoint de publicación solicitado por el usuario2026-09-30: commit y push de
todo el trabajo actual, sin declarar terminada la macro. Workers congelados;
artifacts/PRE locales preservados y excluidos de fuente publicada. Exports de
melee original y GameObjectUseValues integrados por el parent; estos últimos
conservan los nueve préstamos sobre el estado per-Session. BeforeObjects conserva
plan/admisión/permisos en los mismos puntos terminales; seis casos nuevos escritos.
Loot original dispone de seis casos Map y diecinueve APP escritos; conserva ROOT,
TARGET, outcome/batch/cursor0 y usa el algoritmo de generación APP compartido.
No activa el driver ni resuelve hooks/tail/disposición/segundo productor.
La revisión integral Character86/Loot62 y el finalizador birth canónico continúan
pendientes. Se actualiza Cargo.lock offline: sólo paquetes/aristas locales,
sin cambios de versiones de dependencias de registro. La campaña de publicación
se registra sobre el candidato commiteado; implementación escrita no equivale
a aceptación. Este checkpoint no reduce el objetivo global ni cierra#1233/#584.

El recuento por nombres de paths distingue 234 541 líneas en rutas denominadas
tests/fixtures y 174 353 en las restantes; éstas pueden contener tests inline.
Es una clasificación de navegación concurrente, no atribución semántica de
producción/pruebas. `src` incluye tests privados: no presentar sus 347 752 líneas
como código productivo. B3 preserva escenarios APP; la reducción global requiere
llevar las reglas y sus pruebas al owner real, no cambiarles sólo el directorio.

**Power effects, contrato completo fijado e implementación asignada:** parent
lee propuesta, APP energize/drain/burn/charges y los dos writers de carga actuales.
Contrasta EffectDrain/Burn/Energize/Pct, Unit::EnergizeBySpell/ModifyPower y
SpellHistory::RestoreCharge en a5f8da2e. PRE completo, 16 fuentes Rust, y ledger
28 APP originales/101 assertions en `.codex/power-effects-presnapshot-20260930/`.
Unit asume max→requested→current→clamp→write/applied y raw drain; wow-spell asume
spell-ID valuation con engineering lazy; SpellHistory asume loaded/pop/remove y
loop de restore sin cambiar su owner. APP conserva consultas, clocks, amenaza,
execute-log, packets, async damage y las dos rondas de cargas separadas/max.
Mantener interrupción/envío antes del rechazo de max, requested threat frente
a applied log, zero-drain Creature true/energize0 frente a Player false y
Creature values→log→burn await. No fusionar closures ni reparar los gaps C++.
Einstein entrega consumidores/tests completos; parent integra el snippet concreto
de export SpellPowerAmount/SpellPowerGain y dependencia World→wow-spell. Lee los
tres owners, effects_power completo y los 28 casos nuevos completos: 17 Domain/72
macros y once APP/52, escritos sin ejecución. Compara independientemente los 28
originales literalmente, las 43 funciones anteriores de SpellHistory literalmente
y cinco fuentes completas sin cambios, contra PRE íntegro. Informe en
`/tmp/sol-power-original-source-comparison-20260930.json`. Root real Unit es
`unit.rs`, wrapper World `spell_state/catalog.rs`; el delivery conserva sus gaps.
No state/construction, aceptación ni activación. Los 28 originales permanecen APP;
los nuevos owner/lazy/negativos son adicionales. Einstein prepara el lifecycle
completo de Creature seleccionado para la retirada del segundo productor B6.

**Reconstrucción completa Spell/Skill entregada e integrada, sin aceptar:** PRE íntegro en
`.codex/spell-skill-login-presnapshot-20260930`, con 22 fuentes Rust y ocho
funciones C++ según inventario de Einstein. Parent lee los algoritmos de
dependencies/ranks/gain/downgrade, setter row/slot y los doce originales/27
assertions. Fija tres cortes privados: cursor específico de reconstrucción en
PlayerSpellRuntimeState (buffer conocido prestado, sin nuevo owner), operación
LearnedSkill de gain/downgrade y transición row/slot en PlayerGameplayState.
Los lookups y writers APP conservan sus puntos y fallos; no fusionar closures.
Gain mantiene cobertura estricta y writes anteriores ante fallo; downgrade
mantiene búsqueda previous→node→previous→first→node y clamps, incluso value300
con max75 o máximo cero ante facts ausentes. Flags dependent/favorite preceden
membership/append/enqueue; overrides siguen después, incluidos auto/alreadyknown.
No seen/gate cero/cycle repair ni adquisición de autoridad nueva. Login mantiene
autoridad/finalización/publicación después del pase de skills. Los doce casos
pasan al owner y los nueve Remove siguen atravesando la operación de unlearn
existente; tests APP wire/equipo/fallo/invalidation y scenario23 siguen completos.
CPP `a5f8da2e`: AddSpell3038–3083/3100–3119, Remove3296–3363, SetSkill5635–5852.
La expansión antes de skills y la proyección de ranks son gaps actuales respecto
a C++, no reparación autorizada. Einstein entrega los tres cortes, adapters,
doce originales/27 assertions y veinte nuevos (doce Domain/ocho APP). Saldo
declarado World: −56 producción/−316 tests; Domain producción 569, sin contar
el traslado físico de los dos casos equipment como retirement adicional.
Parent lee los cuatro children Domain completos, adapters recon84/learned132,
setter/instalación row-slot, soporte con UnlearnOperation real, los doce casos
trasladados y los veinte nuevos. Comparación literal independiente de los
63 métodos y 40 otros casos/131 assertions declarados aún pendiente.
Tras ACK explícito de Pauli (cuatro roots libres, sin edición en vuelo), parent
integra el parche concreto de Einstein en spell_runtime.rs,
player_gameplay_state.rs, player/mod.rs y lib.rs. Montajes/reexports presentes;
turno devuelto a Appearance. No compilación, typing ni pruebas ejecutadas.

**Límite global aún abierto (recuento estático de archivos `.rs`,
2026-09-30, durante la barrida):** `wow-world/src` y `wow-world/tests` suman 409 294 líneas;
123 545 pertenecen a `src/session/tests`. El árbol tiene ediciones concurrentes;
el recuento es una instantánea de navegación. Mover escenarios entre esos dos
directorios no reduce el total del crate. No se ha alcanzado el objetivo de
200 000 líneas ni el de owners lógicos: además de B3/C/D, B4 debe retirar
responsabilidades completas de Session y llevar reglas y sus pruebas al
owner correspondiente. Estos números son navegación, no aceptación ni
medición de rendimiento. No cerrar la macro por cumplir solo techos físicos.

**B4, reglas trasladadas y siguiente familia:** `wow-combat` ya contiene
absorción física/maná/curación, armadura y bonus melee hecho/recibido, con sus
modelos resueltos y pruebas puras. La aplicación conserva selección de
catálogos/auras, writes canónicos, RNG y publicación. Los IDs de aura tienen
una sola definición en constantes, reexportada desde datos. Se preservan los
gaps heredados de truncado separado de AP y cheat-death 45182 ausente; no se
reparan dentro del movimiento. Contraste `a5f8da2e`, `Unit.cpp:1810-1934`,
`:2024-2069`, `:7558-7777`; `SpellAuraEffects.h:365-407`.
Bonus de spell y regla de hostilidad trasladados, sin aceptar. Zona/área aplica cada etapa en una
mutación de Player; conserva muestra espacial, caché, setters y contraparte
del snapshot. Delta explícito: desaparece la ventana entre closures de Session,
sin acreditar paridad de movimiento. Once slots de items pasan al holder existente; Unit ya secuencia regeneración con la misma closure y publicación posterior, sin aceptar (`Player.cpp:1609-1678`).
Diez policies sociales de fixtures trasladadas. Elegibilidad de quests pasa al estado canónico: 12 tests originales de dominio y nueve de aplicación; se conserva prioridad DF sobre daily y checks independientes de peers exclusivos. CalculateReputationGain y sus stages compartidos con quests pasan a progression, conservando consultas lazy, guards y casts; diez casos nuevos escritos. Revisión de fuente terminada, sin aceptación. Contraste de zona: `Object.h:527`, `Object.cpp:999`, `Player.cpp:1016/7298/7356`, `Player.h:3010`, mismo checkout.

**C1/C2, implementación provisional (2026-09-29, sin aceptar):**
`wow-data-model` reúne esquemas inmutables de GameObject, Creature, Quest,
Pet, Power, Vehicle, JumpCharge, facción/amistad/paragon/currency y spillover.
Las rutas antiguas reexportan una sola definición; los stores/loaders quedan
en datos. El dominio ya no necesita `entities/movement→data` productivo y datos
no declara `data→entities/movement`. Los 28 tests puros de GameObject y los de
CurrencyTypes acompañan al modelo; el contraste DB2/Player usa dev-dependency.
La autoridad completa de loot y su coordinación `sync::watch` residen en
entidades; no se autorizan nuevos tasks, timers o I/O por esa dependencia.
Los escenarios privados permanecen con el owner. Lockfile, targets, concurrencia
y paridad se comprobarán en la campaña final; estos movimientos no cierran C1/C2.

**C3/C4, integración provisional (sin aceptar):** MonsterMove y CreateObject
reciben valores preparados por los adaptadores de `wow-world`; `wow-packet`
ya no declara `wow-movement`. El writer conserva la secuencia de campos y
bits revisada; la proyección de CreateObject mantiene el clamp del tiempo
antes de decidir JumpExtra, incluida la prueba nueva de tiempo negativo.
Esa excepción y `anticheat→packet` se retiran tras revisar consumidores; anticheat recibe flags/elevación, conserva nueve tests y los gaps de seguridad GM/fuzzyEq (`Player.cpp:28412-28509`, `a5f8da2e`), sin aceptación. CreatureCreateData pasa a Model con 44 campos idénticos; Packet sólo cambia reexport, writer/ocho casos conservados. Dependencia y excepción Packet→Entities retiradas tras revisar fuente.
`xtask check-layers` y `check-deps` delegan en la ruta `dependencies` de la
política canónica, con propagación de errores; los baselines numéricos quedan
como historial, no como una segunda autoridad. Las pruebas de routing y
fallos están escritas, pero no ejecutadas. No aceptar C3 por revisión estática
de bytes ni C4 por el mero retiro del evaluador antiguo.

**Siguientes cortes semánticos de capa, contratos fijados (sin aceptar):**
`wow-map` conserva lectura de terreno y fallback de área; recibe un callback
estrecho de padre de subzona, evaluado una vez después de resolver el área.
El adapter privado de aplicación conserva el lookup DB2 y exactamente el filtro
de `ParentAreaID`/`IsSubzone` (`TerrainMgr.cpp:671-677`, `a5f8da2e`). El único
otro consumo normal de datos es el ID de taunt, trasladado a constantes con la
familia de aura; el test de integración DB2 permanece con dependencia de
desarrollo. Esto permite retirar `map→data` sin mover ni duplicar el catálogo.
En reputación, `wow-progression` produce valores de publicación y conserva
las mutaciones de `need_send` y `send_faction_increased`; el adapter de
aplicación construye los tres paquetes después de liberar el acceso canónico.
Se preservan el orden de facciones, prioridad de la principal, standing visual,
defaults, los 1 000 índices y el orden actual de reset y envío. Contraste:
`ReputationMgr.cpp:324-384` y `ReputationPackets.h:29`, mismo checkout. Retirar
`progression→packet` no cierra su dependencia restante hacia catálogos concretos.
Ambas unidades tienen implementación provisional y revisión estática de los
consumidores: el mapa conserva `wow-data` sólo como dependencia de desarrollo;
reputación ya no declara ni consume `wow-packet` ni `wow-data`. Tras revisar
sus ocho consumidores y el view prestado, se retiran las tres aristas obsoletas. El mapa conserva casos negativos
de padre cero, entrada ausente y fila no subzona; los tres converters de
reputación tienen contrastes de bytes escritos y no ejecutados. No se afirma
compilación, aceptación ni paridad a partir del retiro de aristas.

**D1/D2/D4/D5, continuación física provisional:** Player mantiene un solo
owner; lifecycle, descanso, construcción, inventario y update fields tienen
módulos privados. AreaTrigger conserva transiciones en su raíz de 705 líneas.
Los catálogos de arranque retienen Arc, orden y diagnósticos en fases privadas;
la app mide 3 018 líneas; ConditionMgr/PlayerChoice quedan en fases de 205/453, SpellWorldStartup en 297, SpellPetStartup en 248, WorldObjectStartup en 302, WorldStateStartup en 75, progresión en 200, criaturas en 225 y AreaTrigger SQL en 212. Supervisión/apagado queda en 352, con owners y orden actuales. El adapter DB de lifecycle delega sus dos
cargas al mismo owner: raíz 851 y child 775, sin cambiar SQL/resultados.
Su aceptación terminal exige revisar el inventario exhaustivo de persistencia;
`--syntax-only` no lo sustituye. `character/identities.rs` conserva un enum
canónico de 1 791 líneas, 600 identidades estáticas y `GENERATED_CPP`.
Su excepción específica tiene techo sin crecimiento de 1 791, checkpoint
`1233:D4-terminal-identity-review` y vencimiento 2026-10-29; no cubre dispatch
ni SQL. Loot fixtures: fachada 42 y children de 354/285/165/194, misma feature.
El [detalle fechado](workspace-structure-history.md#detalle-de-cortes-revisados-el-2026-09-29-sin-aceptación)
conserva fuentes y medidas. Ninguno de estos recuentos acredita aceptación.

**Reparto de implementación activo (2026-09-30, sin validaciones intermedias):**
el parent integra y revisa; seis workers Luna max tienen scopes separados.
Retomar por los archivos reales y reconciliar los procesos antes de reasignar.

| scope | unidad en curso y siguiente límite |
|---|---|
| B4/combat | Player posee readiness (13 casos) y lote de daño (ocho); Map posee refs bilaterales (10) y limpieza CombatStop (ocho). LootViews (nueve), ItemAccess (diez), ResponseView (dos originales más cinco nuevos), construcción/partición de pools (dos originales/10 assertions más ocho nuevos) y viewer state (ocho) revisados. Currency gain (ocho más cuatro de wiring), reputación de quest (13 más seis de wiring/nueve externos), white swing (ocho), health transitions (12) y commit de absorción (siete) revisados. Aura queries: 14 funciones/21 métodos, cuatro schemas idénticos y únicos en Model, un original/ocho assertions más diez nuevos. Créditos NO-item/threshold: seis APIs/12 casos/siete de wiring, 19 originales intactos. Quest lifecycle 162/391: seis originales/16 assertions más diez nuevos, 12 operaciones/13 bindings revisados. Ingress 357/493: tabla original más nueve casos. Ballots 111/276: nueve nuevos, originales 6/9/14 conservados. Resurrection vitals 76/259: 13 nuevos/cinco integraciones conservadas. Aura installation 496/463: 18 nuevos/un original trasladado y 282 líneas de producción retiradas de World, source revisado. Aura removal 145/253: ocho operaciones/12 nuevos revisados, originales APP conservados. Derived stats 297: cuatro schemas/Defaults/métodos idénticos en Model y 11 originales/76 assertions idénticos tras redirección de llamada; sin Data→Entities. Quest hydration 133/270: 16 nuevos/tres APP nuevos/siete originales APP conservados, source revisado. Disenchant 233/492: tres originales/11 assertions y 12 nuevos/105 assertions; builder, RNG lazy, referencias/cap/orden y APP revisados en fuente. Retirada completa de spells contratada/asignada al PlayerSpellRuntimeState: cursor específico frames/seen, catálogos lazy y escritores/publicaciones APP en sus puntos actuales; nueve Domain/21 assertions migran y 19 APP/77 permanecen. Conserva fallo por frame, closures e invalidaciones separadas de prev-rank, sin snapshot del runtime ni nuevos tasks/locks. C++ RemoveSpell:3236–3462, a5f8da2e; gaps heredados preservados. Sin aceptación ni cierre global. |
| D1/Player | Player 845, Aura 964, runtime/map 940 y fixtures 304/418/497/117/344; mismos 63 nombres/mounts. RealmList 248, respawn bootstrap 290, writer DB 291 y supervisión de sesiones 196 separados; lib 949, fields privados conservados. Retry/mailbox, flags Acquire/Release, wakeups y shutdown revisados. El timeout de blocking sigue siendo un gap; sin aceptación. |
| D2/app | Loot 264 conserva nueve cuerpos; factory 200 devuelve los 27 slots. Creación 119/SpellInfo 142 conservan cuatro/nueve Arc. ConditionMgr 205 y PlayerChoice 453 preservan loads/logs/contextos/informes. App 2 519 tras DatabaseStartup 138: cuatro aperturas/cuatro validaciones ordenadas, nueve adapters World entre World y Hotfix y guards de esquema/orden preservados en fuente; compositor completo asignado con raíz/children ≤600. SpellWorldStartup 297, supervisión 352 y AreaTrigger SQL 212 revisados. SpellPetStartup 248/WorldObjectStartup 302 mantienen ports/reports; WorldStateStartup 75 mantiene dos llamadas/tres awaits/cuatro Arc/dos clones DB. Progresión 200 mantiene cinco awaits/15 Arc/dos clones DB e informes completos. CreatureCatalogStartup 225 conserva dos fases/10 awaits/11 Arc/dos clones Hotfix DB y dos source guards migrados. GeographyStartup 187 conserva seis awaits/11 Arc/dos clones DB e informes. GUID bootstrap 152 conserva seis awaits/cuatro Arc/guard hasta shutdown y cleanup COMMIT antes de Item. GroupStartup 52/WorldInstanceStartup 112 conservan dos awaits/ocho Arc/siete clones e informes completos; caches posterior intacto. Handler policies 159 conserva 28 getters/tres Arc/cero clones/cero awaits y guard adaptado al child real. PlayerCatalogStartup 104 conserva diez bindings/dos awaits/ocho Arc/cero clones/cuatro logs/10 contextos y fishing-before-tiers. Sin compilación ni aceptación. |
| D4/database | 600 expresiones SQL originales idénticas. B6 conserva un mapa en vuelo y plan/diff/epoch/incarnation/origen; consumidor mantiene fases adyacentes bajo guard. WorldCreature agrupa 24 campos runtime (dos casos). ObjectRef/ObjectMut revisados (cinco); ObjectEntry transporta Record completo (siete). OwnedMapObject/snapshots y ActorStorage revisados (diez/once casos). Familias movement y terreno comparten motor pending (15 nuevos); Step 254/172/339 y witness/admisión 84/290/459 revisados (diez nuevos cada unidad). ObjectWork 117/280 conserva wrapper/seis nuevos. Barrera tick/finalizers y 21 casos nuevos revisados en fuente; receipt terminal exige sesiones/ticks/finalizers vacíos antes del tick final/mailbox. Acceso token/slot y Actor movement revisados, 15 casos cada uno y exports integrados. APP movement 405/360 y once casos revisados; disposición owned entregada con siete casos Map/nueve APP nuevos escritos. Cuerpos Map 77/APP 74, exports y 16 casos revisados en fuente, sin ejecución. Admisión Fresh 84/manager 33 y 14 casos nuevos revisados en fuente; mantiene incoming en errores/duplicados y mueve motor intacto. AddToMap 778 byte-idéntico frente a PRE íntegro; preflight cubre los gates actuales antes de efectos. Partición física Map entregada: raíces inserción/removal/relocation/storage 125/351/223/107, 14 children (máximo399). Parent compara 122 funciones originales: 121 cuerpos literales; coordinador reconstruido desde tres stages equivale ignorando whitespace. ObjectEntry byte-idéntico frente a PRE íntegro. Transferencia viva/promoción separadas. Respawn contratado/asignado: store común con slots SavedOnly/Catalog/Actor, un ejecutor por clave, INFO e índices derivados, Instant/ordinal y namespace GUID-low conservados. Preparar transferencia move-only bajo quiescencia; dos instancias temporales quedan explícitas hasta retirar legacy. Delta de carrera y semánticas de startup/remove/load/fallo en B6-execution; sin activación ni reparación oculta. Aggro completo contratado y en implementación: queries/facts de catálogos en APP, mutaciones seleccionadas/recíprocas en Map y LOS owned fuera del guard; sin avance de clock/RNG ni activación. Contrato por fases/mapa y deltas de scheduling en B6-execution. Retirada de segunda entidad/productor y aceptación pendientes. |
| C1/modelos | GameEvent 41/827/112/467 conserva 69 métodos; Hotfix 367/313/567 conserva 13 loaders/19 converters/cuatro tests. WorldStateMgr global puro 192 en entidades; adapter 162 conserva SQL/CSV/reportes y siete tests. POI revisado en fuente: tres schemas idénticos en Model, agrupamiento Data con cuatro casos, conversión/caché World y writer Packet idéntico. Packet→Model permitido expresamente, sin excepción; gaps SQL/clone conservados. Sin aceptación. |
| C/progression | Reputación conserva ocho consumidores/37 tests con view prestado y sin datos/packet normales. Instances usa cuatro esquemas compartidos y lookups perezosos detrás de guards; downscale sigue en datos y sus pruebas pasan al adapter real. Arista instances→data retirada tras revisión estática; gaps/orden conservados, sin aceptación. |

Este reparto no acredita cierre de B3/B7, reducción global a 200 000 líneas,
retirada de los dos productores/managers de B6 ni aceptación de la macro.

**B6, frontera de ejecución pendiente (revisión estática 2026-09-29):**
`world-server/app.rs` inicia el loop canónico y el global legado con intervalos,
`Instant`/diff y epochs independientes. La barrera World/Map pertenece al loop
canónico; no ordena por sí sola el bridge legado. Su orden interno es
PlayerMelee → Lifecycle → Movement → Aggro → Spell → Melee. El envelope legado
captura mapas, incarnations y GUIDs del manager canónico, pero las fases aún
enumeran el manager legado; el envelope no es una autorización de trabajo.
El mismatch de incarnation se calcula después de publicar y sólo se informa.
El resume canónico mantiene respawns antes de ObjectUpdater y termina con
SendObjectUpdates/DelayedUpdate bajo su propio plan. Unificar los productores
requiere una frontera que libere los guards antes de ejecutar trabajo externo
y retenga el mismo plan hasta su tail, con workset e incarnations efectivos,
fences DB y cancelación/shutdown resueltos. Añadir la llamada al final del tick
o borrar el loop legado no satisface ese contrato. Contraste:
`World.cpp:2701/2748`, `MapManager.cpp:287-318`, `Map.cpp:666-785/2191` y
`Creature.cpp:696`, checkout `a5f8da2e`. No se ha cambiado scheduling ni
retirado ningún bridge. El respawn legado inserta antes de intentar su contraparte
canónica; evento unspawn y unload no limpian ambas caras. El [contrato B6](workspace-structure-b6-execution.md)
fija continuación, gaps y vida de closures antes de filtros; no acredita paridad o aceptación.

**Recursos del host (2026-09-29):** se retiraron únicamente los directorios
inspeccionados de compilación inactiva de `target/{debug,release}` del checkout
principal (`deps`, `build`, `incremental`, `.fingerprint`), recuperando unos
12 GiB; quedan unos 39 GiB libres. Se preservó íntegra la caché activa del
worktree, además de binarios de despliegue, capturas y logs. Los dos procesos
vivos siguen usando `target/deploy/live`; no hubo reinicio ni cambios de runtime.
Estas cifras son una observación fechada, no una reserva de espacio.

## 4. Qué significa "verde" en cada nivel (no confundir niveles)

1. **Compila**: evidencia de `cargo check -p <crate>` y consumidores cuando comienza
   la campaña autorizada. Durante esta barrida también se difiere el feedback de
   workers: no se lanza por helper, módulo o entrega local. Un check solo no acepta la macro.
2. **Suite**: los tests del crate afectado, y si se movieron tests, los invariantes contados
   (`#[test]`/`#[tokio::test]`, `inventory::submit!`, lista de escenarios).
3. **Composición**: `cargo check -p world-server --tests` y los targets de integración.
4. **Aceptación**: la campaña final única. Solo aquí se habla de aceptado.

Ninguna fase se declara cerrada con el nivel 1; la tabla de estado registra el nivel alcanzado.

## 5. Reglas para no perdernos

1. **Una fase, un commit**, con su mensaje diciendo qué cambió y qué se verificó.
2. **Movimiento y comportamiento separados**, siempre.
3. **Nunca** subir un techo, regenerar una política o retirar un test para que pase una fase.
4. Si una fase revela trabajo no previsto, **se añade a este documento** antes de hacerlo.
5. Al cerrar cada ola, se reejecuta la auditoría y se actualiza §1 y §3.
6. Nada se publica (push/PR/merge) sin autorización explícita.

## 6. Decisiones arquitectónicas

Los [ADR aprobados](workspace-structure-decisions.md) conservan sus contratos y fechas; ADR-009 rige los símbolos nuevos de esta barrida, cuya corrección está asignada, sin renombrado masivo del legado.
Los [diagnósticos y resultados anteriores](workspace-structure-history.md) son evidencia histórica;
la continuación se mantiene exclusivamente en §3.1 de este programa.
