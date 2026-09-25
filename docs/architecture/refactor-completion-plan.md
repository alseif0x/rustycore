# Plan técnico para completar la arquitectura de RustyCore

**Sincronización de la entrega #748, F1/#61/#63 y PR #933/#931/#929/#927/#925/#924/#923/#922/#921/#919/#918/#917/#916/#915/#913/#911/#909/#907/#906/#904/#902/#901/#899/#897/#895/#893/#891/#889/#887/#885/#881/#878 — 2026-09-15; actualización #524 genérico, SQL hotfix, locale y P2/P3.13/P3.12/P3.11/P3.10/Transport VALUES/VehicleKit/Battleground/persistent-capabilities/world-local/taxi/item-object/item-modifier/void-storage — 2026-09-14.** Este documento detalla los
límites técnicos de la dirección general que mantienen `docs/migration/PORT_PLAN.md`
y GitHub #49. No es un plan de issues alternativo: el índice macro, sus lanes y sus
dependencias viven en el plan de port; aquí se fijan propietario, consumidores,
anclas C++, orden de ejecución y criterios de aceptación de la arquitectura.

La forma objetivo de crates/capas, los presupuestos duros y la secuencia de fases de la
distribución de `wow-world` se detallan en
[wow-world-distribution-plan.md](wow-world-distribution-plan.md); este documento sigue siendo el
plan técnico general y aquel no lo sustituye.

## #1233 — descomposición de Session y reglas independientes, 2026-09-22

Estado: **implementación local en curso, no validada ni publicada** sobre
`9daa13f663bd1e863a3efed06721c3fcb3b6cd66`, en la rama
`1233-archcore-modularize-wow-world-session-and-isolate-ruletest-boundaries`.
La petición explícita mantiene nivel 1 para la aceptación: no se han ejecutado
tests, campañas arquitectónicas ni QA. Se ejecutó sólo un `cargo check -p wow-world`
diagnóstico en un target dedicado; terminó correctamente con advertencias, por lo
que el SHA sigue sin ser un candidato de aceptación.
La macro no cierra #584, #29 ni el experimento de medición #1231.

### Fronteras implementadas

- `session/mod.rs`: de 18.908 a 1.002 líneas físicas. Los tipos y adaptadores
  anteriormente definidos en la raíz se distribuyen en 65 módulos privados por
  responsabilidad: conexión, admisión, Player, NPC/GameObject, inventario,
  progresión, pet, quest, publicación de combate y contratos de persistencia.
  Sigue existiendo **un solo `WorldSession`**, definido en `session/state.rs`
  (1.994 líneas); su construcción está en `session/construction.rs` (1.185).
  Es organización física, no retirada de campos ni migración de autoridad.
- Los miembros privados trasladados declaran `pub(in crate::session)` para
  conservar exactamente su antiguo ámbito efectivo. No se hacen públicos a
  handlers externos, otros crates o composición. La fachada conserva las rutas
  anteriores; no cambia el thunk ni la fuente única de registro de opcodes.
- `session_tests.rs`: de 5.806 a 389 líneas. Sus 152 montajes de escenarios
  permanecen en esa raíz; los builders compartidos pasan a 15 módulos privados
  bajo `session/tests/fixtures/`, con acceso limitado a `session::tests`.
- `wow-combat`, antes reservado y vacío, recibe la tabla melee determinista,
  ensamblado de probabilidades desde hechos del llamador y cálculo del daño
  por resultado. Sus módulos son `attack_table`, `facts` y `damage`; no tiene
  dependencias. `wow-world` pasa a consumirlo como dependencia de producción.
  `session_rules/rules_4.rs` conserva la tirada RNG, la presentación de paquetes
  y la mitigación dependiente de auras. Sus reexports son un puente de importación,
  no una segunda implementación: se retirarán al migrar los consumidores de la
  familia completa de `session_rules`, sin obligar a mezclar esa tarea ahora.
- Cuatro tests puros de `scenarios_combat_4`, `scenarios_combat_5` y
  `scenarios_world_entities_32` se trasladan a `wow-combat/tests/melee_contract.rs`.
  Dos casos adicionales ejercitan el API público de daño en `melee_damage.rs`.
  La regresión existente de daño/presentación y los escenarios con owners
  canónicos siguen en `wow-world`; **ninguno se ha ejecutado en este corte**.
- `wow-spell-acquisition` recibe el planificador completo de adquisición de
  hechizos/skills, sus modelos y autoridades de evidencia. Es un componente de
  **aplicación determinista**: toma prestados los catálogos canónicos de
  `wow-data`, no depende de Session, transporte, paquetes ni base de datos.
  No se fuerza esa dependencia concreta dentro del crate de dominio reservado
  `wow-spell`, ni se copian los catálogos ni se introduce un trait por store.
  La política declara el límite y restringe sus dependencias internas a
  `wow-core` y `wow-data`; el acoplamiento a datos concretos sigue explícito.
- Sus 63 escenarios focales existentes se trasladan con el planificador. Tres
  escenarios que cruzan hacia preparación/profesiones quedan completos en
  `wow-world/src/spell_acquisition/tests/planner_application.rs`; `operations`
  y los tests de aplicación/runtime/persistencia también permanecen en world.
  Los builders de metadatos se comparten mediante `test-fixtures`, sin feature
  por defecto y habilitada sólo como dependencia de desarrollo de world.
- La lista causal de publicación de `SpellAcquisitionPlanLikeCpp` sigue privada.
  La aplicación recibe un accessor de sólo lectura; no puede reemplazar la
  autoridad para hacer coincidir acciones inventadas. Las pruebas negativas
  conservan sus valores mediante un builder de desarrollo y un mutador sólo
  disponible con `test-fixtures`/`cfg(test)`. Se añade una regresión de aislamiento
  de esa lista y un doctest `compile_fail` para su privacidad; no están ejecutados.
- `wow-conditions` recibe la familia completa de evaluación `ConditionMgr`,
  contextos, consultas de spell-click/loot y sus 40 escenarios. Sus módulos
  privados se separan por responsabilidad; conserva el único
  `OnceLock<RwLock<Option<Arc<ConditionEntriesByTypeStore>>>>` existente, sin
  nuevo lock ni segunda copia del store. También es una frontera de aplicación
  con lectura de los catálogos actuales, no un dominio independiente de datos.
- Ambas bibliotecas están conectadas en Cargo y en los consumidores. Las
  fachadas anteriores de world sólo reexportan la implementación única; se
  conservan para la compatibilidad de los adaptadores y consumidores externos
  (incluido el bootstrap de world-server). No se incluye el código mediante
  `#[path]` cruzando crates. Retirar las fachadas requiere migrar esos imports,
  no volver a extraer ni duplicar la lógica.

El recuento físico local de este corte pasa de **425.675 a
414.214 líneas Rust en `wow-world`**: **11.461 líneas netas menos**.
Los dos nuevos crates contienen 8.226 y 3.452 líneas,
respectivamente, incluidas sus pruebas. Son unidades de compilación separadas,
pero siguen compartiendo dependencias inferiores; esto no demuestra un ahorro
de segundos, ni hace barato todo cambio transversal en datos o en Session.

### Contrato y evidencia consultada

Referencia local TrinityCore 3.4.3 en
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`:
`Server/WorldSession.cpp`, constructor y `LogoutPlayer`/`SetPlayer`;
`Entities/Unit/Unit.cpp`, `DoMeleeAttackIfReady`, `CalculateMeleeDamage`
(1300–1443), `RollMeleeOutcomeAgainst` (2272–2383),
`GetUnitDodgeChance`/`GetUnitParryChance`/`GetUnitBlockChance` (2639–2760),
`GetUnitCriticalChanceAgainst` (2819) y `MeleeSpellMissChance` (11652–11684);
`Entities/Player/Player.cpp`, `GetBlockPercent` (25288–25298).

Para adquisición: el mismo `Player.cpp`, `AddSpell` (2741), `LearnSpell` (3192),
`SetSkill` (5635) y `LearnSkillRewardedSpells` (23930). Se conserva el orden de
mutaciones/proyecciones, el cierre ante metadatos incompletos, los límites de
trabajo y la distinción entre aprender directamente y ejecutar un wrapper.
Para condiciones: `Conditions/ConditionMgr.cpp`, constructores de
`ConditionSourceInfo` (160/176), `Condition::Meets` (194),
`IsObjectMeetToConditionList` (997), `IsObjectMeetToConditions` (1040–1052) e
`IsObjectMeetingSpellClickConditions` (1156). Se trasladan los casos soportados
y no soportados existentes, sin convertir esta reorganización en nueva prueba
de paridad. Ningún writer de Player, transacción ni publicación cambia de dueño.

El traslado conserva las fórmulas Rust existentes, incluido el término de
crushing del target, el truncado y el orden de las bandas. No pretende reparar
lagunas preexistentes de cobertura de combate ni demostrar paridad completa.
La inmunidad conserva el resultado representado existente; no se afirma que
el retorno temprano C++ asigne `OriginalDamage` igual que este helper.
La RNG sigue en el mismo wrapper; no se añaden reloj, task, lock, estado mutable,
SQL, paquete ni nueva ruta de publicación. Los comentarios obsoletos sobre
ausencia de bloqueo de Player/ExpectedStat se corrigen a la frontera real de
entradas resueltas por el llamador, sin alterar las fórmulas.

### Aceptación pendiente y continuación

#### Continuation: character login ownership and directory delivery, level 1

This implementation slice remains an uncommitted structural refactor on top of
`9daa13f663bd1e863a3efed06721c3fcb3b6cd66`. The pinned TrinityCore 3.4.3 source
is `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`.

- `handlers/character/world_entry/initial_packets.rs` owns the ordered packet
  bursts on either side of canonical map insertion; `login_recovery.rs` owns
  homebind and failed-login recovery. `items/login_load.rs` owns the login
  inventory/item restoration snapshot. The orchestration and phase order remain
  in `world_entry/login.rs`. Source anchors are
  `src/server/game/Handlers/CharacterHandler.cpp` and
  `src/server/game/Entities/Player/Player.cpp`: `Player::SendInitialPacketsBeforeAddToMap`,
  `Player::SendInitialPacketsAfterAddToMap`, `_LoadInventory`, `_LoadVoidStorage`,
  `_LoadEquipmentSets`, and `_LoadTransmogOutfits`.
- Effective-stat snapshot/update helpers now live in `character/stats_update.rs`;
  the read-only level-up query remains in `stats_queries.rs`. Existing
  calculations, canonical-owner updates, and packet publication were moved
  without changing their contracts.
- Character handler support code was colocated with its consumers across the
  gossip, query, lifecycle, visibility, pet, and world-entry modules. The
  character-family source scan in `character_tests/loot.rs` now includes the
  extracted child modules so its existing publication-order assertions still
  inspect the whole family. Relevant target anchors include
  `Handlers/NPCHandler.cpp::HandleGossipSelectOptionOpcode` and
  `Handlers/CharacterHandler.cpp`'s login sequence.
- The vendor C++-slot resolver remains private in `character/vendor.rs`; only
  vendor stock bookkeeping is in `vendor_admission.rs`, avoiding a wider
  resolver visibility than its callers require. Homebind repair and its
  persistence helper are owned by `world_entry/login_recovery.rs`, called from
  the existing login orchestration. This is a structural move only; the
  represented fallback and persistence order are unchanged. Target anchors are
  `CharacterHandler.cpp::HandlePlayerLogin` and
  `Player.cpp::Player::_LoadHomeBind` at the pinned source revision above.
- `character/items/destruction.rs` now groups `handle_destroy_item`, its quest
  persistence planner, and the recursive full-stack destruction helpers.
  Temporary-enchantment cancellation remains in `items.rs`. The child retains
  existing effective method visibility, and the family publication-order scan
  in `character_tests/loot.rs` includes the new source. Target anchors are
  `ItemHandler.cpp::HandleDestroyItemOpcode` and `Player.cpp::DestroyItem`;
  persistence and runtime mutation order were not changed.
- Loaded-item row normalization, enchantment/socket construction, and item-field
  application helpers moved from the character facade to
  `character/item_load_support.rs`; `items/login_load.rs` remains the hydration
  operation owner. Existing helper paths are retained by a private facade import.
  The item-family scan includes this module as well. The source anchors are
  `Player.cpp::_LoadInventory` and `_LoadVoidStorage`; no hydration order or
  canonical item authority changed.
- Account-wide collection loaders and the login spell projection moved to
  `character/account/collections.rs`. `world_entry/login.rs` still owns the
  call sites and their execution order; `character_tests/loot.rs` now includes
  the child in its whole-character-source publication scan. The methods retain
  their previous effective scope through `pub(in crate::handlers::character)`.
  C++ anchors are `Player.cpp::LoadFromDB` (17711–17715), which invokes
  `CollectionMgr::LoadToys`, `LoadHeirlooms`, `LoadMounts`,
  `LoadItemAppearances`, and `LoadTransmogIllusions`, plus the corresponding
  `LoadAccount*` routines in `CollectionMgr.cpp` and `Player.cpp::SendKnownSpells`
  (2536). One existing ordering difference is retained explicitly: Rust currently
  loads toys, heirlooms, appearances, illusions, then mounts; C++ loads mounts
  before appearances and illusions. This move does not reorder calls or claim
  parity for that difference.
- Character enumeration and its test adapter moved to
  `character/account/enumeration.rs`; the `EnumCharacters` `PacketHandlerEntry`
  remains in `account.rs`, and the whole-character-source scan now includes the
  child. The pinned C++ anchors are `CharacterHandler.cpp::HandleCharEnumOpcode`
  (407–425) and `HandleCharEnum` (326–405). The Rust persistence-port call,
  ban-cleanup error handling, row projection, and failure/success responses were
  moved without changing order; no new parity claim is made for the represented
  character fields or race-unlock data.
- Quest-source item storage moved from the reward module to
  `handlers/quest/source_items.rs`; quest acceptance still calls the same
  `WorldSession` method. This separates the grant performed during quest
  acceptance from final reward selection. Target anchors are
  `QuestHandler.cpp::HandleQuestgiverAcceptQuestOpcode` and the
  `Player.cpp::Player::AddQuest` → `Player::GiveQuestSourceItem` path. The
  existing persistence and quest-update sequence was moved without alteration.
- Final reward inventory storage and currency grant helpers now live in
  `handlers/quest/rewards/items.rs` and `rewards/currencies.rs`. The reward
  orchestrator and durable-plan boundary remain in `rewards.rs`; admission
  remains in `rewards/validation.rs`. Parent-called helpers retain the same
  effective scope, with `pub(super)` only where delegation now crosses into a
  child module. Target anchors are `QuestHandler.cpp::HandleQuestgiverChooseRewardOpcode`,
  `Player.cpp::RewardQuest`, `Player.cpp::RewardQuestPackage`, and the durable
  contract in [quest-reward-operation-contract.md](quest-reward-operation-contract.md).
  Commit participants, recovery, and publication order were not changed.
- `session/directory.rs` remains the sole `PlayerRegistry` storage and snapshot
  owner. Current-incarnation packet/command delivery and durable runtime
  publication methods moved to `session/directory/delivery.rs`; feature-gated
  fixture accessors and directory tests moved to `test_fixtures.rs`. The public
  method names, registration generations, queue/error behavior, and canonical
  authority are unchanged. This is an internal Rust organization boundary, not
  a new gameplay-parity claim.

The loot source family was also separated by canonical source: creature
generation, corpse-lifetime checks, and creature projections now live in
`handlers/loot/sources/creature.rs`; the gameobject/gathering/fishing code stays
in `sources.rs`. Six `cfg(test)` request adapters moved to
`handlers/loot/requests/test_support.rs`. Loot-window admission and represented
view/cache operations remain in `requests.rs`; direct item-storage operations
are grouped in `requests/item_storage.rs`, with disenchant winner/batch
operations in `requests/item_storage/disenchant.rs`. The existing method scope
and caller paths are preserved.

The storage split was checked against the pinned TrinityCore source
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`:
`Handlers/LootHandler.cpp::HandleAutostoreLootItemOpcode` (77–150),
`Entities/Player/Player.cpp::StoreLootItem` (25643), and
`Loot/Loot.cpp::LootRoll::Finish` (575–620). It is organization-only: loot
generation, claim admission, persistence participants/recovery, packet order,
and publication semantics were not changed; no new parity claim is made.

Quest packet-handler bodies were also split without moving or changing registrations:

- Quest-giver acceptance and shared-quest confirmation methods now live in
  `handlers/quest/handlers/acceptance.rs`. The pinned C++ anchors are
  `QuestHandler.cpp::HandleQuestgiverAcceptQuestOpcode` (105–227),
  `HandleQuestConfirmAccept` (499–531), and `Player.cpp::AddQuestAndCheckCompletion`
  (14241), `AddQuest` (14398), and `GiveQuestSourceItem` (15448). Existing
  checks, state transitions, persistence and packet order were moved unchanged.
- Quest-giver dialog/status and quest query methods now live in
  `handlers/quest/handlers/queries.rs`. Their source anchors are
  `QuestHandler.cpp::HandleQuestgiverStatusQueryOpcode` (41–74),
  `HandleQuestgiverHelloOpcode` (76–103), `HandleQuestgiverQueryQuestOpcode`
  (228–254), and `HandleQuestQueryOpcode` (255–268), plus
  `QueryHandler.cpp::HandleQueryQuestCompletionNPCs` (252–278) and
  `HandleQuestPOIQuery` (280–298). The existing Rust quest-giver query path
  remains its represented response only; the C++ auto-accept behavior was not
  added by this structural move.
- Quest request-reward, complete-quest, and choose-reward handlers now live in
  `handlers/quest/handlers/reward_flow.rs`. Their C++ anchors are
  `QuestHandler.cpp::HandleQuestgiverRequestRewardOpcode` (410–438),
  `HandleQuestgiverCompleteQuest` (533–590), and
  `HandleQuestgiverChooseRewardOpcode` (269–408). The existing reward
  transaction, unknown-COMMIT recovery, and publication contracts were moved
  unchanged; [quest-reward-operation-contract.md](quest-reward-operation-contract.md)
  remains the operation authority.

All anchors above use the pinned TrinityCore revision
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`. `PacketHandlerEntry` registrations
remain in `handlers.rs`, including the exact source read by
`QUEST_HANDLER_REGISTRATIONS`; opcode metadata, thunk calls, and tests were not
changed.

Static inventory for the quest handler-method split:

```text
wc -l crates/wow-world/src/handlers/quest/handlers.rs crates/wow-world/src/handlers/quest/handlers/{acceptance.rs,queries.rs,reward_flow.rs,sharing.rs}
468 crates/wow-world/src/handlers/quest/handlers.rs
551 crates/wow-world/src/handlers/quest/handlers/acceptance.rs
285 crates/wow-world/src/handlers/quest/handlers/queries.rs
483 crates/wow-world/src/handlers/quest/handlers/reward_flow.rs
984 crates/wow-world/src/handlers/quest/handlers/sharing.rs
2771 total
```

Static inventory command and result for the directory/loot ownership split:

```text
wc -l crates/wow-world/src/session/directory.rs crates/wow-world/src/session/directory/{delivery.rs,test_fixtures.rs} crates/wow-world/src/handlers/loot/requests.rs crates/wow-world/src/handlers/loot/requests/test_support.rs crates/wow-world/src/handlers/loot/requests/item_storage.rs crates/wow-world/src/handlers/loot/requests/item_storage/disenchant.rs crates/wow-world/src/handlers/loot/sources.rs crates/wow-world/src/handlers/loot/sources/creature.rs
1827 crates/wow-world/src/session/directory.rs
307 crates/wow-world/src/session/directory/delivery.rs
323 crates/wow-world/src/session/directory/test_fixtures.rs
341 crates/wow-world/src/handlers/loot/requests.rs
141 crates/wow-world/src/handlers/loot/requests/test_support.rs
884 crates/wow-world/src/handlers/loot/requests/item_storage.rs
678 crates/wow-world/src/handlers/loot/requests/item_storage/disenchant.rs
1283 crates/wow-world/src/handlers/loot/sources.rs
753 crates/wow-world/src/handlers/loot/sources/creature.rs
6537 total
```

The item-destruction physical split is independently counted with:

```text
wc -l crates/wow-world/src/handlers/character/items.rs crates/wow-world/src/handlers/character/items/destruction.rs
1575 crates/wow-world/src/handlers/character/items.rs
433 crates/wow-world/src/handlers/character/items/destruction.rs
2008 total
```

The character facade/item-restoration support split is counted with:

```text
wc -l crates/wow-world/src/handlers/character/mod.rs crates/wow-world/src/handlers/character/item_load_support.rs
1797 crates/wow-world/src/handlers/character/mod.rs
209 crates/wow-world/src/handlers/character/item_load_support.rs
2006 total
```

Character account operation ownership was statically counted with:

```text
```
wc -l crates/wow-world/src/handlers/character/account.rs crates/wow-world/src/handlers/character/account/{collections.rs,enumeration.rs}
1436 crates/wow-world/src/handlers/character/account.rs
321 crates/wow-world/src/handlers/character/account/collections.rs
185 crates/wow-world/src/handlers/character/account/enumeration.rs
1942 total

The quest reward and source-item ownership splits are counted with:

```text
wc -l crates/wow-world/src/handlers/quest/rewards.rs crates/wow-world/src/handlers/quest/rewards/{items.rs,currencies.rs} crates/wow-world/src/handlers/quest/source_items.rs
892 crates/wow-world/src/handlers/quest/rewards.rs
532 crates/wow-world/src/handlers/quest/rewards/items.rs
126 crates/wow-world/src/handlers/quest/rewards/currencies.rs
413 crates/wow-world/src/handlers/quest/source_items.rs
1963 total
```

No tests, builds, formatters, architecture checks, or QA were run for this
continuation under level 1. The code remains dirty and has no candidate SHA;
`HEAD` is still the base SHA above. Implementation status is therefore
unvalidated. The login orchestration and remaining #1233 ownership work are
still in scope; this slice does not complete the macro.

#### Continuación: distribución de loot e inventario, nivel 1

Corte de extracción, no reparación de gameplay: las operaciones receiver-free
de permisos, recuento, reconstrucción y consumo por jugador de
`handlers/loot/mod.rs` pasan a un módulo privado `wow-loot::distribution`, junto
al `CreatureLoot` que ya usan. API de funciones con los mismos argumentos y
resultados; ningún nuevo estado, lock, task, catálogo o dependencia. Los
adaptadores conservan sus puntos de llamada, autoridad/generación, guardas de
claims, orden commit/aplicación/publicación y paquetes. No cambia ningún opcode.
Anclas en el mismo C++ fijado: `Loot/Loot.cpp`, `FillNotNormalLootFor` (1024),
`hasItemForAll` (950), `hasItemFor` (963), `hasOverThresholdItem` (984),
`AutoStore` (820–898); `Entities/Player/Player.cpp`,
`Player::isAllowedToLoot` (17963–18006). Se preservan las representaciones Rust existentes,
sin afirmar nueva paridad ni unificar helpers de la vista y claims con contratos
distintos. Los tests de paquetes/Session/DB se quedan en world; los casos de
estado puros se trasladan y se añaden casos focales de reconstrucción/consumo.
Los imports en la raíz de handlers son un adaptador privado, no otra implementación.
Se reutilizan las seis constantes de método de loot que ya pertenecían a
`wow-loot`, retirando sus duplicados en world. Se trasladan completos los casos
`represented_unlooted_count_counts_shared_items_once_like_cpp` y
`creature_loot_visibility_applies_full_cpp_allowed_to_loot_gate`. Los cuatro
casos nuevos cubren FFA consumido, reconstrucción de loot compartido, slot
inexistente y la precedencia de pertenencia/threshold; son código de prueba
escrito, no evidencia ejecutada.
Aceptación diferida: suite de distribución y suite de autoridad de wow-loot,
regresiones de loot de world y consumidores de publicación/persistencia.
No se ejecutan bajo nivel 1; no se declara el runtime listo para QA viva.

El mismo corte traslada la política de tiradas a `wow-loot::rolls`: máscara de
votos permitidos y resolución parcial/final, con el DTO de voto existente.
Sólo cambia la entrada de los dos resolutores de `&RepresentedLootRollState`
a `&HashMap<ObjectGuid, RepresentedLootRollVote>`; reciben el mismo mapa, sin
copiarlo ni ordenar sus claves. Se conserva el desempate por primer elemento
encontrado con esa iteración, la precedencia Need y la espera por votos pendientes.
Timeout, RNG, identidad de la tirada, autoridad/generación, premios y paquetes
permanecen en Session. Anclas: `Loot.cpp`, `LootRoll::TryToStart` (398–452),
`PlayerVote` (454), `AllPlayerVoted` (520) y `UpdateRoll` (498);
el premio/publicación de `Finish` (575) no se mueve. No se afirma que el orden de un HashMap
sea un nuevo contrato de paridad entre lenguajes; se conserva el comportamiento
Rust representado y sus pruebas de integración. Se escriben seis casos locales
de máscara, prioridad, votos pendientes, ausencia de ganador y desempate; no ejecutados.

En inventario, `wow-entities::player::items::storage_move` recibe la validación
del plan posterior a la asignación de huecos y sus dos registros de decisión.
Es un módulo privado con reexports explícitos desde la biblioteca existente;
no se añade crate ni dependencia. La API recibe el `PlayerInventoryItem`
canónico, cantidades, destinos `ItemPosCount` ordenados y tres consultas
síncronas concretas: item de destino, cantidad del objeto y tamaño de stack.
Mantiene la consulta/rechazo temprano, suma comprobada, fallback de stack a 1,
un único remanente y los errores representados actuales. Los campos públicos
son valores de la decisión, no exposición del Player ni una nueva autoridad.
No se añade detección de destinos duplicados ni se repara gameplay implícitamente.

`WorldSession::plan_inventory_storage_move_like_cpp` conserva la búsqueda del
origen y `CanStoreItem`/`CanBankItem`; ejecución, cercas de persistencia, quest
y paquetes siguen intactos. Tampoco se mueve ni se multiplica el clone previo
de `direct_inventory_player_snapshot`: esa deuda permanece fuera de este corte.
Anclas: `Handlers/ItemHandler.cpp`, `HandleAutoStoreBagItemOpcode` (699–753);
`Handlers/BankHandler.cpp`, `HandleAutoBankItemOpcode` y
`HandleAutoStoreBankItemOpcode`; `Entities/Player/Player.cpp`, `CanStoreItem`,
`CanBankItem`, `StoreItem`/`_StoreItem` y `BankItem`. El no-op del allocator Rust
conserva `InternalBagError`; no se afirma que sea el error de todas las rutas
C++ de banco (`HandleAutoBankItemOpcode` distingue `CantSwap`). No se mezcla
una corrección de esa diferencia con este traslado.

Los escenarios existentes de Session/banco permanecen en world. Se escriben
once casos de biblioteca para merges, remanente, orden de consultas, fuente,
GUID/entry, objeto inexistente, límites de stack, sumas y errores de destino.
Aceptación diferida: esos casos de wow-entities, los escenarios existentes
`bank_move_plan_*`/`autostore_bank_move_plan_*` y las regresiones de persistencia,
aplicación y publicación de ambos handlers. Nada de ello se ejecuta en nivel 1.

Recuento físico tras el corte de loot/inventario: **413.684 líneas Rust en 838 archivos de wow-world**,
incluidos los targets de integración, frente a 425.150 en la base Git
`9daa13f6`: **11.466 líneas netas menos respecto a esa base**. El recuento
425.675 → 414.214 de arriba corresponde al corte local anterior, no a dos
SHAs validados. Comandos de inventario de tamaño (no compilación ni aceptación):

```bash
rg --files crates/wow-world -g '*.rs' -0 | xargs -0 wc -l | awk '$2 != "total" {lines += $1; files++} END {print files, lines}'
git grep -c '^' HEAD -- 'crates/wow-world/**/*.rs' | awk -F: '{lines += $NF} END {print lines}'
```

En esta continuación se reutilizan wow-loot y wow-entities: cero crates nuevos,
cero dependencias nuevas. Los seis archivos nuevos suman 1.277 líneas incluidas
las pruebas, con 23 casos (dos trasladados y 21 nuevos), todos sin ejecutar.
Mover tests reduce la suite alojada en world; el planificador genérico de
inventario aún puede monomorfizarse en el consumidor. Estos recuentos no son
mediciones del trabajo del compilador ni prueba de mejora de tiempo.

#### Continuación: reglas y transiciones de objetivos de quest, nivel 1

Se elige una extracción hacia wow-entities ya existente, no otro crate ni una
dependencia inversa hacia wow-data (que produciría un ciclo). `QuestObjective`
y sus dos operaciones de valor se trasladan completos a
`wow-entities/src/quest_objectives/model.rs`; `wow-data::quest::QuestObjective`
reexporta ese mismo tipo. El catálogo, la carga, normalización, relaciones y
`QuestTemplate` permanecen en wow-data. Su nuevo `objective_rules_like_cpp`
construye una vista inmutable con el mismo slice ordenado, sin asignar ni clonar
objetivos. La dependencia existente data → entities se conserva, no se declara
resuelta la deuda general de separación de datos.

Las siete reglas anteriores de `handlers/quest_rules.rs` pasan a `completion`
y `items` bajo ese módulo privado. Los tres planificadores reciben una consulta
síncrona de definición por ID; se mantienen los puntos de lookup, la iteración,
los clamps, el orden por slot/ID para bound items y las listas de resultados.
Los consumidores de handlers, Session y planificación de persistencia llaman
directamente a wow-entities, sin un segundo cuerpo ni fachada world transitoria.
Se centralizan sólo las constantes de estado/objetivo implicadas en
`wow-constants::quest`; las rutas públicas existentes en conditions/data y los
aliases privados de handlers conservan los mismos valores.

Además se trasladan las dos transiciones de mutación item/bound-item desde las
closures del handler al `PlayerQuestGameplayState` canónico, en su hijo privado
`player/quest_state/objectives.rs`. Devuelven registros de cambios y candidatos
a completar: **no ejecutan ni adelantan la fase de completar la quest**. Se
conservan el guard de mutación, invalidación de autoridad, selección del orden,
consultas previas, calls async de completion, snapshots de persistencia y cada
publicación en world. Los clones previos de snapshots/rewarded IDs no se amplían
ni se declaran retirados. No hay nuevo estado, lock, tarea, reloj ni opcode.

Anclas en TrinityCore `a5f8da2e`: `Quests/QuestDef.h`, `QuestStatus` (140),
tipos/flags de objetivos (330/357/371), `QuestObjective`/`IsStoringFlag`
(442/477); `Conditions/ConditionMgr.cpp`, validación del límite de
`CONDITION_QUEST_OBJECTIVE_PROGRESS` (2598–2614); `Entities/Player/Player.cpp`,
`CanCompleteQuest` (14123), `ItemAddedQuestCheck` (16067),
`ItemRemovedQuestCheck` (16088), `UpdateQuestObjectiveProgress` (16181),
`IsQuestObjectiveCompletable` (16475), `IsQuestObjectiveComplete` (16533) y
`IsQuestObjectiveProgressBarComplete` (16608). Se conservan las limitaciones
del Rust representado: criterios comparados con amount en este helper, fuentes
vivas no soportadas que devuelven false y timer expresado como end_time no nulo.
No se usa el traslado para reparar esas diferencias ni probar paridad completa.

Se trasladan tres tests puros conservando nombres/casos, se añaden casos de
planificación/mutación canónica y uno del adaptador de datos. Los escenarios
existentes de banco/loot/source-item, estado de quest, commit/aplicación y
paquetes siguen en world. Aceptación diferida: suites de quest_objectives y
quest_state de wow-entities, suite quest de wow-data, suite wow-conditions y
regresiones world de item-objectives/persistencia/consumidores world-server.
No se ha ejecutado ninguna bajo nivel 1; no hay candidato probado ni publicación.

Estado físico de este corte: **412.561 líneas Rust en 848 archivos de wow-world**,
4.261 menos que el recuento tras loot/inventario y 15.725 menos que la base Git
9daa13f6. La fachada `handlers/quest_rules.rs` se elimina; el módulo nuevo de reglas
se separa en modelo, completion, items, resources y pruebas (los módulos de
producción quedan por debajo de 500 líneas; el fichero agrupador de pruebas sigue
siendo una excepción de fixture). Las transiciones canónicas ocupan un hijo de
194 líneas y sus tests un hijo de 265 líneas. Se retira el montaje/archivo `quest_tests/misc.rs` que quedó
vacío tras mover sus dos escenarios; ambos siguen declarados en la suite nueva.
La familia conserva los casos trasladados y añade regresiones de reglas, estado,
admisión, transición por umbral y adaptación de datos; todos siguen sin ejecutarse.
Formato aplicado con
`rustfmt --edition 2024 --config skip_children=true` a las rutas editadas,
sin `--check`. El recuento usa el mismo comando físico documentado arriba.
El `cargo check -p wow-world --message-format=short` diagnóstico terminó en
35,82 s con código 0 y 278 advertencias; no sustituye la campaña final.
Después de corregir los errores de las rutas `cfg(test)`,
`cargo test -p wow-world --lib --no-run` terminó en 3m30s y la ejecución
`cargo test -p wow-world --lib` registró 3.930 tests: 3.929 pasaron, uno quedó
ignorado y ninguno falló. Esta evidencia cubre la suite de librería de world,
pero no sustituye las suites de los crates nuevos, integración de producción ni
los checks arquitectónicos.
La auditoría `check_architecture.py check` y su self-test (20 pruebas) pasan; el
checker sintáctico de ownership pasa con 225 campos de producción, 429 de fixture,
74 owners de implementación y 3.934 asociados exactos. La campaña
`validation-v2 final --base origin/3.4.3 --timings` alcanzó 949,598 s y agotó su
límite de 900 s durante el `cargo check --workspace --all-targets`; el manifiesto
queda como evidencia fallida por timeout, no como aceptación terminal.
Este estado es un avance dentro del **refactor completo de wow-world**, no su
aceptación terminal. Quedan los demás eventos de progreso/eligibilidad/recompensa,
los adaptadores y las demás familias señaladas por #1233; no se reduce el objetivo.

El contador monotónico `game_time_ms_like_cpp` se mueve a
`session::time_synchronization`, conservando el único `OnceLock<Instant>`, el
`wrapping` de `u32` y la misma escala de milisegundos usada por regeneración,
casts, transporte, battleground, visibilidad y sincronización de reloj. Todos los
consumidores de producción, handlers y fixtures pasan por la fachada de Session;
no se introducen relojes ni estado duplicados.

La regla de tombstone no durable de skills (`is_non_durable_skill_tombstone_like_cpp`)
queda junto a sus modelos en `session::player_spell_records`; progresión y adquisición
mantienen el mismo predicado bajo `cfg(test)` y dejan de depender de `session_rules`.

El generador de GUID de battle pets y su contador de test se agrupan en
`session::battle_pet_adapter`; el único consumidor de jaula usa esa fachada y
se mantiene el mismo `AtomicI64`, orden relajado y `HighGuid::BattlePet`.

La validación receiver-free de `creature_message_to_set_target_allows_like_cpp`
se agrupa en `session::world_entities::creature`, junto a los consumers de
mensajes de criaturas. Mantiene los filtros de visibilidad, mapa/instancia,
phase shift y distancia estricta 2D/3D; no cambia la admisión ni el paquete.

El predicado de test que documentaba que los hechizos de monturas de cuenta no se
guardan en `character_spell` se inlinea en su único escenario; se elimina otro
helper sin consumidores de `session_rules` y se conserva la misma afirmación
respaldada por `CollectionMgr::AddMount`/`Player::_SaveSpells`.

Los restos de `session_rules/rules_1.rs` y `rules_2.rs` se eliminan: el primero
ya sólo contenía dos índices de `CombatRating`, ahora propiedad de
`session::combat`, y el segundo estaba vacío. Los consumidores conservan la
fachada de Session y no se altera ningún índice ni cálculo de estadísticas.

`handlers/character/items.rs` deja de contener los cuatro handlers de equipment
sets: save, assign-spec, delete y use pasan a
`handlers/character/items/equipment_sets.rs`. La impl sigue siendo de un único
`WorldSession`, con las mismas validaciones, mutaciones, persistencia diferida y
paquetes; sólo se reduce el hotspot físico del handler principal.

La familia de movimientos de inventario de ese mismo handler se extrae a
`handlers/character/items/inventory_moves.rs`: validación de destinos, planificación
de swaps/equipamiento, ejecución de merges y publicación de posiciones conservan
el mismo `WorldSession` y sus contratos internos. El archivo principal baja a
2.192 líneas y el submódulo queda en 1.563; `cargo check -p wow-world` pasa tras el
corte y no se introduce una nueva autoridad de inventario.

En loot, la interacción y publicación de estado de GameObject se extrae a
`handlers/loot/sources/gameobject.rs`: apertura de cofres, pesca y nodos de
recolección, admisión de visibilidad y refrescos de estado siguen siendo métodos
del mismo `WorldSession`. `sources.rs` baja a 2.312 líneas y el submódulo queda
en 810; la suite completa de `wow-world` conserva 3.929 tests pasados, uno
ignorado y cero fallos.

El flujo post-instance de login se extrae de `handlers/character/world_entry.rs`
a `handlers/character/world_entry/login.rs`. El facade queda en 305 líneas y el
hijo conserva la única secuencia C++-ordenada de hidratación/publicación (2.489
líneas), registrada como excepción física transitoria con salida en #584:C4 hasta
que pueda modelarse un contexto de fases sin alterar orden ni ownership.

La operación de compartir quests se extrae de `handlers/quest/handlers.rs` a
`handlers/quest/handlers/sharing.rs`. El facade queda en 1.766 líneas y el nuevo
módulo en 961, manteniendo la misma admisión, evidencia sender-local y contratos
de publicación; `cargo check -p wow-world` y el checker de ownership siguen pasando.

La proyección de objetos para condiciones se extrae de
`handlers/character/session_state.rs` a `handlers/character/condition_objects.rs`.
Los cuatro builders de `WorldObject` y snapshots de `wow-conditions` conservan
la misma impl de `WorldSession`, visibilidad y política fail-closed; no se añade
autoridad, espejo, lock ni reloj. El adaptador de estado queda en 2.621 líneas,
el módulo nuevo en 101, `cargo check -p wow-world`, los 57 tests filtrados de
condiciones y el checker arquitectónico pasan.

La suite completa de `wow-world` queda verde en la candidata actual: 3.929
tests pasados, uno ignorado y cero fallos con `--test-threads=1`. La flakiness
de los escenarios de melee se corrigió en los fixtures: el aura que cancela la
banda de miss usa `MOD_HIT_CHANCE`, que es el campo que el runtime de criaturas
resuelve para el atacante; las cinco repeticiones de los escenarios afectados y
la suite completa pasan. Esto no sustituye la campaña `validation-v2 final`, que
previamente quedó pendiente por timeout del chequeo workspace.

La reconciliación de la caché de ventanas de loot y la asignación de GUID de
`LootObject` se extraen de `handlers/loot/requests.rs` a
`handlers/loot/request_cache.rs`. El módulo principal queda en 2.418 líneas y
el hijo en 122; la autoridad canónica y el fallback exclusivo de fixtures se
mantienen intactos. `cargo check -p wow-world`, los 328 tests filtrados de loot
y el checker arquitectónico pasan.

La admisión de stock/refill y la resolución BFS de slots de vendedor se extraen
de `handlers/character/vendor.rs` a `handlers/character/vendor_admission.rs`.
El facade conserva las compras, reparaciones y trainer gossip; la nueva impl
mantiene los mismos contratos de persistencia y fallback `cfg(test)`. El módulo
principal sale del hotspot físico de 2.519 líneas; `cargo check -p wow-world`,
los 30 tests filtrados de vendor y el checker arquitectónico pasan.

Las consultas puras de estadísticas de personaje (`player_stat_changes` y
`level_up_stat_deltas`) pasan a `handlers/character/stats_queries.rs`; los
actualizadores que mutan salud/poder y publican paquetes permanecen en
`session_state.rs`. El corte conserva la misma autoridad Player y el test de
nivelación pasa junto con `cargo check` y el checker arquitectónico.

Los comandos de combate de criaturas (`AttackStart`, `AttackStop` y la
reconciliación de expiración PvP) pasan de `handlers/loot/handlers.rs` a
`handlers/loot/combat_commands.rs`. Siguen siendo entregas acotadas al
`WorldSession`, sin mutar la autoridad del Map; `cargo check`, seis escenarios
de combate y el checker arquitectónico pasan.

Los cuatro comandos de publicación sensible a visibilidad (`UpdateObject`/
`UpdateObjectValues`, `SendIfVisible`, el par `SpellStart`/`SpellGo` de criaturas
y la entrega condicionada de addons) pasan a
`handlers/loot/visibility_commands.rs`. El módulo sólo conserva las compuertas
finales de sesión y la entrega de bytes ya serializados; la selección de
destinatarios, la autoridad del estado y la identidad map/instancia siguen en
sus propietarios existentes. Este corte es estructural y queda sin validar bajo
nivel 1.

La orden de refresco de visibilidad de criaturas queda en el mismo módulo. Sólo
comprueba la sesión y la identidad map/instancia antes de invocar el recorrido
canónico existente; no recrea el conjunto visible ni mueve la autoridad del Map.

Las tres órdenes de sincronización de estado de GameObject (nodo de recolección,
cofre y goober) pasan a `handlers/loot/sources/gameobject.rs`, junto a sus
operaciones de apertura y refresco. Conservan el mismo filtro de mapa/instancia,
decodificación de `LootState`/`GoState`, snapshot de estado y llamada al refresco
visible; no se introduce una segunda autoridad de GameObject.

La carga de filas/referencias de condiciones de loot de criaturas, su evaluación
representable, la admisión por jugador y la lectura de `item_template_addon`
quedan en `handlers/loot/sources/creature_conditions.rs`. Es una separación de
consulta y admisión: no mueve la generación, el claim, el almacenamiento ni la
autoridad de la tabla de loot.

El contexto de grupo/jugador de las peticiones de loot y sus compuertas de
objetivos de quest se extrae a `handlers/loot/requests/context.rs`. Mantiene el
snapshot de Player o del registro remoto, la selección de looter de dungeon, el
progreso receiver-free y la carga ordenada de metadatos; los handlers siguen
siendo dueños de la apertura, commit y publicación del loot.

Los dos comandos receptores del flujo de compartir quest (`SetQuestSharingInfo`
y la solicitud de objetos de turn-in repetible) pasan al módulo de sharing de
quest, junto al emisor que los produce. Se conserva el mismo estado pendiente,
GUID receptor y secuencia de detalle; loot deja de ser el propietario físico de
esa transición.

Los comandos de cancelación/estado de trade y las dos publicaciones de duelo
representado pasan al handler social, que ya es el dueño físico de las
interacciones sociales. Se mantienen la compuerta de pareja activa, el reset de
aceptación, el arbiter GUID y los bytes de paquete sin cambiar su orden.

La carga de cooldowns y cargas de hechizos desde las filas auxiliares de login
se separa en `handlers/character/session_state/login_data.rs`. Conserva la
limpieza previa, los filtros contra los catálogos, la autoridad de estado de
hechizos y el orden de los paquetes; sólo cambia la ubicación física del método.
El mismo módulo recibe la carga de `TraitConfigs`/`TraitEntries`, incluida la
normalización, los gates de completitud y la autorización exacta de hechizos de
traits; no se cambia la frontera de autoridad ni la secuencia de login.

La admisión de elecciones de quest-package y la planificación de espacio de
inventario para recompensas pasan a `handlers/quest/rewards/validation.rs`.
El módulo conserva los filtros de plantilla, facción, fallback de paquetes,
errores de inventario y la preflight `CanRewardQuest`; la aplicación, persistencia
y publicación de la recompensa permanecen en `rewards.rs`.

La transacción de recompra de vendedor (`CMSG_BUY_BACK_ITEM`) se separa en
`handlers/character/vendor/buyback.rs`. Mantiene el mismo preflight de slot,
dinero, plan de merge/move, COMMIT único, espejo canónico y publicación de
updates; no cambia la autoridad de inventario ni de persistencia.

La construcción de `DynamicObjectCreateData` desde el snapshot canónico se mueve
a `session::object_updates`, junto a la entrega de actualizaciones de objetos.
Se conservan GUID, entry, flags, escala, posición, caster, spell visual, spell,
radio y `cast_time_ms`; los consumidores de efectos, resolución de mapas y
fixtures usan la misma fachada y se elimina otro helper de `session_rules`.

La admisión receiver-free de objetos de quest también se mueve a
`wow-entities::player_has_incomplete_quest_objective_for_object_id_like_cpp`.
Recibe el snapshot ordenado de estados y una consulta de definiciones; mantiene
el fallback del índice de almacenamiento, el límite mínimo de cantidad y el
fallo cerrado ante catálogo ausente. Session sólo resuelve el catálogo y combina
el resultado con `QuestLogItemId`/item-drop; no conserva una segunda regla.
El ancla C++ es `Player::ItemAddedQuestCheck`/`UpdateQuestObjectiveProgress`
(`Player.cpp:16067`, `16181`). Se añade una regresión pura de admisión; no se
ejecuta todavía.

Los tres recorridos de transición por umbral (dinero, moneda y reputación) comparten
ahora `wow_entities::plan_threshold_quest_objective_changes_like_cpp`. El planificador
es sólo observador: conserva el filtro por estado incompleto/completo, identidad de
objetivo, dirección inversa de `QUEST_OBJECTIVE_MAX_REPUTATION`, bloqueo de una
transición ya completada cuando el valor no retrocede y el orden del catálogo; los
handlers siguen siendo dueños de la mutación de Player, completion async, persistencia
y paquetes. La extracción corresponde a `Player::UpdateQuestObjectiveProgress`
(`Player.cpp:16181`) y a la comparación de objetivos de `Player.cpp:16475–16608`.
Incluye regresiones puras para cruce de umbral y reputación máxima; permanecen sin
ejecutar bajo nivel 1.

La validación receiver-free del nombre de personaje también sale de la capa de
handlers: sus códigos de respuesta se centralizan en
`wow-constants::character` y la regla ASCII/longitud vive en
`wow-entities::represented_character_rename_name_result_like_cpp`. Los handlers
siguen siendo dueños de la autorización, respuesta y persistencia del personaje;
la función trasladada sólo selecciona el código C++. Se conserva una regresión
pura con el orden vacío, corto, largo, carácter inválido y nombre válido. No se
ha ejecutado bajo nivel 1. Las anclas de integración son `CharacterHandler.cpp`
(rename 1533–1538 y customize 1788–1802) y los valores de
`SharedDefines.h:6231–6236`; el chequeo completo de `ObjectMgr::CheckPlayerName`
(`ObjectMgr.cpp:8707`) sigue siendo una diferencia representada preexistente y
no se amplía dentro de este refactor.

El adaptador de condiciones de vendedor deja de duplicarse en world y pasa a
`wow-conditions::is_vendor_item_conditions_with_snapshots_like_cpp`, junto a la
consulta `ConditionMgr` que ya poseía ese crate. Sólo se trasladó la construcción
de `ConditionSourceInfo` desde snapshots; los handlers siguen resolviendo los
objetos, inventario, stock y resultado de compra. El reloj de stock reutiliza
`wow_entities::game_time_secs_like_cpp`; se elimina el último `character_rules.rs`
del world crate. No se ejecutó validación bajo nivel 1.

La admisión de votos de loot ya no pasa por una fachada de reglas del world crate:
`LootRollVoteCommand::targets_identity_like_cpp` vive junto al contrato de mailbox
y compara clave, generación, autoridad y la identidad de la instancia exacta del
roll. El handler sólo consume esa decisión antes de aplicar el voto; los tests de
reemplazo de roll siguen usando el contrato del mensaje. La elegibilidad de tipos
de roll permanece en `wow-loot`; sólo se ha movido la valla de identidad del
transporte, sin alterar el orden de publicación.

La última reexportación de elegibilidad se eliminó también: `wow-world` llama
directamente a `wow_loot::represented_loot_roll_valid_rolls_like_cpp`, por lo que
`handlers/loot_rules.rs` deja de existir. No queda una segunda autoridad para la
selección de tipos de roll.

La regla de cálculo de la ventana de una quest aceptada se incorporó como
`QuestTemplate::accepted_and_end_time_like_cpp(accept_time)` en `wow-data`.
Los handlers muestrean el reloj en su punto de admisión y el modelo aplica
`TimeAllowed` con el mismo `saturating_add`; `quest_rules.rs` conserva únicamente
la clasificación de la fuente de recompensa de moneda, que todavía depende del
enum de publicación de Session. No se alteró la autoridad de persistencia ni se
ejecutó validación bajo nivel 1. La fuente C++ es `Player::AddQuest`
(`Player.cpp:14398–14471`), donde se calcula `EndTime` desde `GetLimitTime()` y
se registra `AcceptTime` con `GameTime::GetGameTime()`.

La clasificación de la fuente de recompensa de moneda sigue la misma frontera:
`CurrencyGainSourceLikeCpp` pasa a `wow-constants::currency` y
`QuestTemplate::currency_gain_source_like_cpp` la calcula junto a sus flags
`Daily`, `Weekly` y `WorldQuest`. Session conserva la aplicación de la moneda,
los límites, la persistencia y el paquete `SetCurrency`; ya no existe una regla
de recompensa duplicada en `wow-world`.

La geometría de caja de area-trigger se trasladó de `session_rules/rules_2.rs`
al owner de datos espaciales `wow-entities::area_trigger`. Session sólo decide
qué trigger consultar y conserva el radio/phase gate; la función pura transforma
la posición a los ejes locales del trigger y comprueba las tres semidimensiones.
Se añade una regresión de rotación de ejes; no se ejecuta bajo nivel 1.

La tasa pura de experiencia de grupo (`xp_in_group_rate_like_cpp`) sale de
`session_rules/rules_2.rs` y pasa a `wow-entities::player_rules`, junto a las
reglas receiver-free de Player. El cálculo del reparto por nivel sigue en
Session porque necesita el snapshot de miembros, mapa y distancia; sólo la
tabla de multiplicadores queda centralizada y reutilizable.

Las dos reglas receiver-free de descanso también salen de `session_rules`: la
normalización finita de `rest_bonus` y la admisión de los estados Rested/Normal/RAF
viven en `wow-entities::player_rules`; sus valores de protocolo se centralizan en
`wow-constants::rest`. Session conserva la carga, las migraciones de filas y la
mutación del estado canónico, sin una segunda validación local.

La comprobación de slots de buyback deja de pasar por `session_rules`: el helper
canónico `wow_entities::is_buyback_slot` se reutiliza desde handlers, persistencia,
equipamiento y almacenamiento. No se modifica la ventana de slots; sólo se retira
la copia receiver-free del world crate.

La tabla `ItemTransmogrificationSlots` se mueve igualmente a
`wow-entities::player_rules::item_transmogrification_slot_like_cpp`. Appearance
mantiene la consulta a stores y la publicación de criterios; la conversión pura
de `InventoryType` a slot de equipo ya no depende de `session_rules`. Se conserva
la regresión de cabeza, arma de dos manos y tipo desconocido.

La conversión de altura de silla a `UnitStandStateType` sigue la misma ruta:
`chair_stand_state_like_cpp` se ubica en `wow-entities::player_rules`; la
interacción de GameObject conserva teleport, ocupación de slot, efectos y
publicación, y sólo delega la tabla pura de estado sentado. Se añade regresión
para la altura base y el overflow que debe caer en `Stand`.

El ajuste porcentual de bonus de descanso (`apply_pct_modifier_to_u32_like_cpp`)
sale del bloque de helpers de test de `session_rules/rules_1.rs` y pasa a
`wow-entities::player_rules`. Mantiene aritmética signed, división entera y
clamp a `u32`; Session sólo conserva el consumo y la mutación del bonus.

Los gates estrictos de distancia 2D/3D se trasladan a `wow-core::position`, junto
a `Position` y sus operaciones geométricas. Creature runtime, ticks de hechizo y
las reglas de visibilidad usan ahora esa autoridad común; la comparación mantiene
la semántica estricta de C++ (`distance² < límite²`) y su regresión de frontera.

La conversión de `SpellEffectInfo::MiscValue` a `area_id` pasa a
`wow-entities::bind_area_id_like_cpp`; el efecto de hechizo conserva la resolución
de zona, posición y publicación. Se mantienen la asignación unsigned de los 32
bits y la regresión para `-1`, sin una regla paralela en `session_rules`.

La clasificación de acciones de encantamiento cargadas que no tienen una
representación directa (`loaded_enchantment_effect_action_is_unrepresented_like_cpp`)
se mueve a `wow-entities::player::item_modifiers`, junto al enum y al runtime que
las produce. El consumidor de Session sólo registra el resultado; se eliminan
la función y la importación duplicadas de `session_rules`.

La segunda clasificación de encantamientos, `represented_item_bonus_action_updates_stats_like_cpp`,
se coloca en el mismo módulo de modificadores. Session conserva únicamente el
recorrido de acciones, la aplicación de efectos y la publicación de cambios; el
helper ya no depende de `session_rules`.

La conversión pura de `UiLinkUseSource::ui_link_type` a `PlayerInteractionType`
(`ui_link_player_interaction_type_like_cpp`) pasa a `wow-entities::game_object`,
junto al modelo de origen de uso. La interacción conserva el envío del paquete y
la publicación de efectos, sin mantener la tabla en `session_rules`.

La tabla de ACK de velocidad (`movement_speed_ack_move_type_like_cpp`) pasa de
`session_rules` a `session::movement_protocol`, junto a `UnitMoveTypeLikeCpp` y
los eventos que la consumen. El manejo de anticheat conserva la misma resolución
de opcode y el mismo rechazo para tipos desconocidos. La tabla complementaria de
publicación (`player_movement_speed_opcodes_like_cpp`) queda en el mismo módulo,
conservando la pareja `Set/Update` por tipo de movimiento.

La regla geométrica `visibility_distance_allows_like_cpp` pasa a
`wow-core::position`, junto a los gates estrictos de distancia. Los consumidores
de efectos, instancias, criaturas, GameObjects y sus escenarios usan la autoridad
común; se conserva la suma acotada de alcances y la comparación estricta.

La clasificación `SpellInfo::IsPositive` (`represented_spell_is_positive_like_cpp`)
sale de `session_rules` y pasa a `wow-data::spell::catalog`, junto a `SpellInfo` y
sus efectos. Los consumidores de aura, amenaza, metadata de criatura y escenarios
usan ahora esa autoridad de datos, conservando los mismos efectos dañinos,
targets enemigos y reglas de signo.

La comprobación de disponibilidad de recursos (`represented_spell_power_has_power_like_cpp`)
también pasa a `wow-data::spell::catalog`. El flujo de casteo conserva sus tres
fronteras (precheck, debit y revalidación bajo el owner guard), pero la regla pura
de costes y snapshot ya no vive en `session_rules`.

La validación recursiva de hechizos aprendidos (`represented_spell_valid_with_seen_like_cpp`)
se mueve al mismo catálogo de datos. Conserva el corte de ciclos mediante `seen`,
rechaza IDs ausentes o triggers no positivos y deja a Session únicamente la
decisión de admisión que consume el resultado.

La tabla pura de candidatos de equipo (`represented_total_avg_equipment_slot_candidates_like_cpp`)
se mueve a `wow-entities::player`, junto a `InventoryType` y los slots canónicos.
El cálculo de nivel medio conserva las reglas de dual-wield, titan-grip y slots
alternativos; Session sólo mantiene la consulta de catálogos y la agregación.

La mutación con guardas de moneda (`plan_remove_currency_like_cpp`) pasa a
`wow-entities::player_gameplay_state`, junto a `PlayerCurrency` y sus estados de
persistencia. Vendor, movimiento y escenarios llaman la autoridad común; se
mantienen el underflow guard, el clamp de cantidad y la transición `New → Changed`.

El planificador de máscara de actualización posterior a retirar de void storage
(`void_withdrawal_post_store_item_values_update_like_cpp`) pasa a
`session::item_modifiers`, junto a los adaptadores de `ItemValuesUpdate`. El
publicador conserva la misma frontera: primero calcula la máscara pura y después
convierte/publica el paquete bajo el estado de Session.

La clasificación de efectos inertes para el perfil de golpe por la espalda
(`player_target_spell_effect_is_hit_inert_like_cpp`) pasa a
`wow-data::spell::catalog`. Aura publication y spell state consultan ahora el
modelo de datos; se conservan el rechazo de triggers y la lista estrecha de
auras/efectos permitidos.

La mutación de reemplazo de un slot de nivel medio
(`represented_avg_total_item_level_maybe_replace_slot_like_cpp`) se mueve a
`wow-entities::player`, junto a la tabla de candidatos. Se conservan el control
de GUID duplicado, el delta saturante y la sustitución sólo cuando mejora el nivel.

La lista de tipos válidos para el seer (`represented_seer_kinds_like_cpp`) pasa a
`wow-entities::object_accessor`, junto a `AccessorObjectKind`. Movimiento,
resolución de instancias y aggro comparten ahora esa tabla canónica sin una regla
duplicada en `session_rules`.

La proyección de los tres IDs de loot de un GameObject (`loot_ids_like_cpp`) pasa
a ser un método del modelo `GameObjectLootSource`. Quest interaction conserva la
resolución y publicación de loot, pero ya no necesita un helper paralelo en
`session_rules`.

El fingerprint de entrega de valores de DynamicObject se mueve a
`session::object_updates`, junto al consumidor de snapshots y la publicación de
updates. `map_key` reutiliza esa única utilidad; `session_rules` deja de contener
hashing específico de la entrega.

La construcción de `UpdateObject` para flags dinámicos de GameObject se mueve al
mismo módulo `session::object_updates`. El consumidor de visibilidad y sus
fixtures usan ahora esa fachada; la conversión de máscara y valores permanece
idéntica.

La máscara de campos para reubicar objetos (`item_storage_fields_values_update_like_cpp`)
se mueve a `session::item_modifiers`, junto al adaptador de void storage. La
publicación y sus fixtures reutilizan el planificador único para contained-in,
dynamic-flags2 y encantamientos modificados.

La conversión de `AuraApplication` a `AuraInfoLikeCpp` (`player_aura_info_like_cpp`)
se mueve a `session::spell_state::aura_publication`, junto a las rutas que envían
actualizaciones de auras. El combate reutiliza la fachada de Session y se
conservan duración, máscara de efectos, cast GUID y puntos escalables.

La transformación de homebind a petición de persistencia
(`player_homebind_update_request_like_cpp`) pasa a `session::persistence::plans`.
Los efectos de jugador y escenarios consumen la fachada de Session; se mantienen
los campos de mapa, área, posición, orientación y GUID del jugador.

La consulta de precios base por nivel (`item_price_base_with_catalogs_like_cpp`)
se mueve a `session::player_items::valuation`, junto al cálculo de valoración de
objetos. La consulta sigue siendo un lookup puro y conserva la pareja armor/weapon.

La conversión de `SendNewItemPlan` a `ItemPushResult`
(`item_push_result_from_send_new_item_plan`) pasa a
`session::player_items::publication`. Persistencia y escenarios reutilizan esa
fachada única; se conservan modificaciones, texto de display, cantidades,
encounter y battle-pet fields.

El GUID centinela de sets de equipamiento (`ignored_equipment_set_item_guid_like_cpp`)
se queda junto a `session::player_items::equipment_sets`, su único consumidor.
Se elimina otra dependencia de `session_rules` sin cambiar la comparación de
GUID ni el recorrido de slots.

Las comprobaciones de opcode de cuenta se localizan ahora en sus consumidores:
heirlooms en `session::lifecycle_ops` y transmog en
`session::player_items::appearance`. Se conserva el rechazo del placeholder
`UpdateCapturePoint`/`0xBADD`, pero desaparece la pareja genérica de helpers de
`session_rules`.

La aplicación de stats calculados de battle pet
(`apply_battle_pet_calculated_stats_like_cpp`) pasa a
`session::battle_pet_adapter`, junto a los tipos de datos que modifica. Los
recorridos de battle pet y journal reutilizan la misma operación, conservando la
actualización de max health, power, speed y health.

La mutación de los flags de visibilidad de Session
(`apply_player_session_visibility_detection_like_cpp`) se localiza en
`session::visibility::operations`. El sincronizador de visibilidad conserva el
mismo owner guard y sólo delega la escritura de los dos flags del `Player`.

La tabla de opcode de spline de criaturas
(`creature_movement_spline_speed_opcode_like_cpp`) se reúne con las demás tablas
de movimiento en `session::movement_protocol`. El consumidor de battle-pet/movilidad
usa la fachada de Session y conserva todos los mappings `Walk`/`Run`/`Swim`/`Flight`.

La lectura de tiempo Unix (`current_game_time_secs_like_cpp`) se elimina de
`session_rules`: los consumidores usan directamente `wow_core::GameTime::now()`.
Se conserva el mismo reloj de segundos y los cierres del owner de descanso reciben
la función de tiempo sin introducir un segundo reloj.

La adaptación local del colector de ownership reconoce la declaración privada
en `session/state.rs` manteniendo la identidad lógica `crate::session`, la
detección de duplicados y el inventario de campos/impls; aún no está validada.
Las rutas y visibilidades
de los snapshots sólo admiten deltas explicados; no se regenera un baseline para
ocultar deriva. La reducción física no cuenta como reducción del owner lógico.

Cuando se seleccione validación: incluir los targets de integración
`wow-combat/tests/melee_contract.rs` y `melee_damage.rs`, las suites afectadas de
Session y sus consumidores, y el colector/políticas arquitectónicas; la aceptación
de publicación conserva sus gates. La reorganización de fixtures y el movimiento
del tracker cambian rutas internas de tests, no su contrato; hay que comprobar
el conjunto ejecutado, no sólo el número. No hay timing antes/después ni ahorro
de compilación demostrado. La medición reproducible sigue perteneciendo a #1231.

La futura campaña debe incluir además las suites y doctests de
`wow-spell-acquisition`, la suite de `wow-conditions`, los escenarios retenidos
de adquisición/aplicación y los consumidores world/world-server. Debe cubrir
tanto la API de producción sin fixtures como la feature de desarrollo y el
store global único a través de la fachada. Cargo/lock/políticas están editados,
pero aún no aprobados por la campaña final. Durante este corte se ejecutaron el
`cargo check` diagnóstico y la suite de librería de `wow-world` descritos arriba;
siguen pendientes las suites de los crates nuevos, integración de producción,
campaña arquitectónica, QA y check de aceptación.

La continuación de inventario/loot/quest aún debe separar las operaciones
restantes de datos, planificación y aplicación sin una dependencia inversa;
los planificadores aquí extraídos no cierran esa familia completa. Runtime/AI
mantiene su orden y autoridad hasta su análisis propio. No se crean crates vacíos
adicionales ni se presenta esta extracción como el refactor terminado de todo
`wow-world`.

Estado exacto tras PR #933 (`ef30bb3e`, código integrado en `ef30bb3e1232c775dcc21ccbc4c91d9196c3e22d`): 649 campos de WorldSession (219 de producción, 430 fixtures). El P4 de navegabilidad separa los tests de loaded-grid en una fachada de producción de 797 líneas y módulos de construcción/resolución. PR #933 separa el runtime de game-events en siete módulos detrás de una fachada de 9 líneas, preservando 181 regresiones. El lock de encuentros, la proyección de visibilidad `Player::m_seer`, la búsqueda de CREATE para Pets canónicas y el DESTROY dirigido genérico de Creature/Pet/Corpse ya usan la autoridad canónica; no queda un residual productivo de WorldSession en este corte auditado. Session conserva un único cerrojo de publicación para el paquete explícito de limpieza FAR_SIGHT.

PR #881 añade el cierre nominal de las mutaciones de aura del Player sobre el `AuraSubsystem`
propiedad de su Unit; la superficie genérica de Session queda limitada a fixtures `cfg(test)`.
PR #876 añade la proyección separada de `Player::m_visibleTransports` para Transport
VALUES a través del registro y del consumidor de Session; no amplía el alcance a
CREATE/DESTROY o al ciclo de pasajeros.

La auditoría C0/C3 del 2026-09-14 seleccionó como macro **Transport CREATE/DESTROY y
visibilidad por fase**, integrado por PR #901 en `3.4.3` (`bf460aa7`). La implementación
captura el Transport typed desde el mapa, resuelve sus datos de CREATE en propiedad,
marca receptores same-phase en add/remove y publica CREATE/OUT-OF-RANGE junto con la
membresía separada de `Player::m_visibleTransports` fuera del guard. Las regresiones
focales de mapa y puente, `cargo check -p wow-world` y fmt/diff pasan. El traslado de
mapa de `Transport::TeleportPassengersAndHideTransport`, seats/offsets, pasajeros,
AI/scripts, taxi y QA viva/DB siguen fuera de este corte y permanecen gates explícitos
de #63/#584.

El **owner de identidad del Player** está integrado por PR #904 (`4ad36d42`).
`name`, `race`, `class`, `level` y
`gender` se resuelven desde `wow_entities::Player`/`Unit`/`WorldObject` después de
la instalación canónica, con un DTO de bootstrap de login de un solo uso. La
superficie antigua de Session queda restringida a fixtures `cfg(test)` y los
consumidores de login de módulos, registro, character, group y chat usan consultas
canónicas. La evidencia C++ es `Player.cpp:17060-17089,17247-17283` y
`Unit.h:733-745`; `m_swingErrorMsg` queda deliberadamente para una macro de melee
separada. El cierre estructural no afirma todavía durabilidad de SaveToDB/relogin, capturas ni
QA viva.

La auditoría C0–C4 posterior a #904 seleccionó el owner de
`Player::m_swingErrorMsg`, integrado por PR #906 en `9a35ba0f`
(implementación `67409023`). La referencia exacta
es `Player.h:3023`, `Player.cpp:20625-20631` y el llamador de melee
`Unit.cpp:2087-2150`: el `Player` canónico conserva el estado nullable y decide
la supresión de duplicados, mientras Session mantiene únicamente la codificación
y entrega de `AttackSwingError`. El owner ausente o obsoleto falla cerrado; no se
introducen espejo de Session, lock, persistencia ni cambio de opcode. El ledger
queda en 647 campos totales de WorldSession (220 de producción y 427 fixtures),
con 8 responsabilidades productivas residuales para auditorías posteriores. Los
checks focales, arquitectónicos y de formato están registrados en el checkpoint;
capturas exactas, DB/relogin y QA viva siguen siendo gates explícitos.

PR #907 integra la clasificación de `represented_confirmed_pending_binds` como
evidencia `cfg(test)` y no como autoridad de Session: C++ confirma el bind desde
`MiscHandler.cpp:1063-1075` sobre Player/InstanceMap. El merge
`33141494b4410a26e01ac5d72ea9f82132b2e522` mantiene el total de 647 campos,
reduce producción a 220 y deja 10 residuos productivos tras la clasificación de
identidad siguiente. Los tres escenarios de `InstanceLockResponse`, `cargo check`
y el ratchet arquitectónico pasan; el path exhaustivo de `wow-world` alcanza el
timeout de 900 segundos y no se presenta como verde.

La auditoría C0–C4 del mismo corte clasifica `recent_player_guid_low_like_cpp`
en la familia cohesiva de identidad de Session. TrinityCore declara
`WorldSession::m_GUIDLow` (`Server/WorldSession.h:1881`), lo actualiza al
adjuntar Player y lo conserva tras logout reciente (`WorldSession.cpp:980-985`),
para atribuir datos de cuenta de personaje (`WorldSession.cpp:877-888`) y admitir
peticiones sociales (`Handlers/SocialHandler.cpp:57-62`). Rust conserva el mismo
writer, lectores y lifetime en `set_player_guid` y el plan de persistencia. Es una
clasificación de ledger sin cambio de código ni comportamiento; quedan 10 campos
productivos residuales exactos.

PR #909 integra la clasificación C0–C4 de `player_guid` en una familia dedicada de
binding de Player en Session. TrinityCore mantiene `_player` en `WorldSession`
(`Server/WorldSession.h:1882`) y lo instala/limpia durante login/logout mediante
`SetPlayer` (`WorldSession.cpp:672-694,978-985`); `set_player_guid` de Rust es la
clave GUID con comprobación de generación para admisión, ciclo de vida,
direccionamiento y rechazo de owners obsoletos. No reconstruye estado de gameplay
ni sustituye al Player canónico. Es una clasificación de ledger sin cambio de
código en el merge `1c8b5577badccb6f74d3b049a7e231494b9fa792`; el residual
productivo exacto queda en 9 campos. Architecture check, self-test (20/20) y diff
validation pasan.

PR #911 integra la clasificación C0–C4 de `vmap_indoor_check_like_cpp` en la
configuración inmutable del mundo, en el merge
`0dbf768419338ac7d3bf3974cf20fa7e7fa4c2ad`. TrinityCore lee `CONFIG_VMAP_INDOOR_CHECK`
desde `World` (`World.h:119`, `World.cpp:1116`); Rust carga e inyecta la misma
clave mediante `SessionRuntimePolicyCapabilitiesLikeCpp`
(`world-server/app.rs:5064`, `session_resources.rs:295,505`) y el consumidor de
aura solo lee el switch. Es una clasificación de ledger sin cambio de código,
paquetes, lock o orden de runtime; el residual productivo exacto queda en 8
campos. Architecture check, self-test (20/20) y diff validation pasan.

PR #913 integra la macro C0–C4 de `represented_cast_unstuck_enabled_like_cpp`
como configuración inmutable del mundo. TrinityCore lee `CONFIG_CAST_UNSTUCK`
desde `World` (`World.h:119`, `World.cpp:1116`) antes de
`Spell::EffectStuck` (`Spells/SpellEffects.cpp:3265-3269`); Rust carga e inyecta
la misma clave mediante `SessionRuntimePolicyCapabilitiesLikeCpp`, y el
consumidor de spell solo lee el policy. La prueba de policy desactivada y la
regresión de configuración cubren la frontera. La clasificación añade el writer
productivo que faltaba, no crea autoridad duplicada ni cambia el orden de runtime;
el merge `9ec36295344d3faf96733b77bac4686d0a903ec1` deja el residual productivo exacto en 7 campos. Los checks focales, arquitectónicos y de formato pasan.

#### Continuation: vendor inventory handler organization, level 1

`WorldSession::handle_list_inventory` moved from
`handlers/character/items.rs` to the private child
`handlers/character/vendor/list_inventory.rs`. Its signature, public method
name, async sequence, callers, and operation body are unchanged; the existing
Spanish inline comments were translated to English. No state, persistence path,
packet field, or gameplay behavior was added or altered.

`account.rs` retains the `ListInventory` `PacketHandlerEntry` with its
existing `LoggedIn`/`Inplace` metadata and thunk. Both direct gossip callers
remain unchanged. The character-family source scan includes the new child, and
`session-ownership-policy.json` records the method's new source module. No
packet registration or test body was moved.

The pinned TrinityCore source is
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`:
`Handlers/ItemHandler.cpp::HandleListInventoryOpcode` (567–573) delegates to
`WorldSession::SendListInventory` (575–697). These anchors establish the
semantic owner and source operation; this structural move makes no new parity
claim and does not repair pre-existing differences in Rust vendor lookup,
interactability checks, movement/home-position updates, pricing, or row
selection.

Static source inventory after the move:

```text
wc -l crates/wow-world/src/handlers/character/items.rs crates/wow-world/src/handlers/character/vendor.rs crates/wow-world/src/handlers/character/vendor/list_inventory.rs
1299 crates/wow-world/src/handlers/character/items.rs
1774 crates/wow-world/src/handlers/character/vendor.rs
285 crates/wow-world/src/handlers/character/vendor/list_inventory.rs
3358 total
```

The preceding worktree inventory was 1,575 lines in `items.rs` and 1,773 in
`vendor.rs`; this move reduces the items file while leaving the vendor root
above the ordinary 1,000-line review threshold. The character/vendor family
remains in progress.

No tests, builds, formatters, architecture checks, or QA were run for this
continuation under level 1. The dirty tree remains unvalidated; `HEAD` remains
the base SHA above and there is no candidate SHA or commit.

#### Continuation: vendor purchase handler organization, level 1

The `WorldSession::handle_buy_item` test adapter and
`handle_buy_item_with_generator_like_cpp` moved from
`handlers/character/vendor.rs` to the private child
`handlers/character/vendor/buy.rs`. Their signatures, visibility, and
`cfg(test)` gate are unchanged. The production handler body was moved intact:
the existing validation, vendor-slot resolution, persistence, canonical
application, and publication sequence was not reordered or expanded. The
shared private `resolve_vendor_buy_item_by_cpp_slot` remains in the parent
vendor module.

`account.rs` retains the `BuyItem` `PacketHandlerEntry` with its existing
`LoggedIn`/`Inplace` metadata and thunk. The transaction-atomicity test
source remains in `character_vendor_atomicity_tests.rs`, and the character
family source scan now includes `vendor/buy.rs`. The item-refund operation
remains in `vendor.rs` and was not changed.

The pinned TrinityCore source is
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`:
`Handlers/ItemHandler.cpp::HandleBuyItemOpcode` (530–564) delegates the buy
request to `Player::BuyItemFromVendorSlot` (`Entities/Player/Player.cpp`,
22338+). This source comparison identifies the operation boundary; the move
does not claim new parity or alter the represented Rust behavior. The Rust
source reference was corrected to the actual C++ method start while moving it.

Static source inventory after the move:

```text
wc -l crates/wow-world/src/handlers/character/vendor.rs crates/wow-world/src/handlers/character/vendor/buy.rs crates/wow-world/src/handlers/character/vendor/list_inventory.rs
817 crates/wow-world/src/handlers/character/vendor.rs
971 crates/wow-world/src/handlers/character/vendor/buy.rs
286 crates/wow-world/src/handlers/character/vendor/list_inventory.rs
2074 total
```

The ownership policy now places both moved methods in
`crate::handlers::character::vendor::buy`; the method signatures, public
visibility, fixture classification, and registration metadata remain
unchanged.

No tests, builds, formatters, architecture checks, or QA were run for this
continuation under level 1. The dirty tree remains unvalidated; `HEAD` remains
the base SHA above, with no candidate SHA or commit.

#### Continuation: real inventory swap operation organization, level 1

`plan_inventory_real_swap_children_like_cpp` and
`execute_inventory_real_swap_like_cpp` moved together from
`handlers/character/items/inventory_moves.rs` to the private child
`handlers/character/items/inventory_moves/real_swap.rs`. Both retain their
`pub(crate)` visibility and signatures. The planner, persistence/application
coordinator, and its existing recursive child-item behavior were relocated
without changing operation order or adding state.

The inventory-move dispatcher remains in its existing module and calls the
same associated method. The `item_2.rs` scenarios that exercise the real-swap
child planner remain in place; the character-family source scan now includes
the new child. No opcode registration or packet handler was moved.

The pinned TrinityCore source is
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`:
`Handlers/ItemHandler.cpp::HandleSwapItem` (130–173) delegates the position
transition to `Player::SwapItem` (`Entities/Player/Player.cpp`, 12271+).
These anchors identify the source operation only; this physical split makes
no new parity claim.

Static source inventory after the move:

```text
wc -l crates/wow-world/src/handlers/character/items/inventory_moves.rs crates/wow-world/src/handlers/character/items/inventory_moves/real_swap.rs
1157 crates/wow-world/src/handlers/character/items/inventory_moves.rs
420 crates/wow-world/src/handlers/character/items/inventory_moves/real_swap.rs
1577 total
```

That measurement precedes the following item-mutation continuation. The
inventory-move family remains in progress.

No tests, builds, formatters, architecture checks, or QA were run for this
continuation under level 1. The dirty tree remains unvalidated; `HEAD` remains
the base SHA above, with no candidate SHA or commit.

#### Continuation: inventory item-mutation organization, level 1

`execute_inventory_equip_to_empty_raw_like_cpp`,
`execute_inventory_auto_unequip_offhand_if_need_like_cpp`, and
`execute_inventory_stack_merge_like_cpp` moved from
`handlers/character/items/inventory_moves.rs` to the private child
`handlers/character/items/inventory_moves/item_mutations.rs`. Each retains its
`pub(crate)` visibility and signature. The dispatcher, real-swap operation,
and equipment coordinator still call the same associated methods; persistence,
canonical mutation, and publication bodies were moved without alteration.

The character-family source scan now includes the child. Existing scenario
sources remain in place, with no test body or packet registration moved.
`session-ownership-policy.json` records all three method paths under
`inventory_moves::item_mutations`.

The pinned TrinityCore source is
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`:
`Handlers/ItemHandler.cpp::HandleSwapItem` (130–173) delegates to
`Player::SwapItem` (`Entities/Player/Player.cpp`, 12271+);
`Player::EquipItem` begins at 11360 and
`Player::AutoUnequipOffhandIfNeed` at 24600. The pinned 3.4.3 source does not
contain `CanEquipChildItem` or `AutoUnequipChildItem`; the related Rust
methods remain in the root module and are not changed or moved here. This
structural slice makes no parity claim for those retained later-upstream
representations.

Static source inventory after this move:

```text
wc -l crates/wow-world/src/handlers/character/items/inventory_moves.rs crates/wow-world/src/handlers/character/items/inventory_moves/real_swap.rs crates/wow-world/src/handlers/character/items/inventory_moves/item_mutations.rs
803 crates/wow-world/src/handlers/character/items/inventory_moves.rs
420 crates/wow-world/src/handlers/character/items/inventory_moves/real_swap.rs
368 crates/wow-world/src/handlers/character/items/inventory_moves/item_mutations.rs
1591 total
```

The root is now below the ordinary 1,000-line review threshold. Planning,
dispatcher, child-item coordination, and position publication remain in the
inventory-move family; this does not complete the wider inventory/quest macro.

No tests, builds, formatters, architecture checks, or QA were run for this
continuation under level 1. The dirty tree remains unvalidated; `HEAD` remains
the base SHA above, with no candidate SHA or commit.

#### Continuation: inventory packet-handler organization, level 1

The item swap, auto-equip, bag-storage, and temporary-enchantment packet
handlers moved from `handlers/character/items.rs` to the private child
`handlers/character/items/handlers.rs`. This includes their existing
`cfg(test)` generator adapters. Method names, signatures, visibility, and
feature gates remain unchanged; the operation bodies were moved without
altering their order or behavior.

`account.rs` retains all six `PacketHandlerEntry` registrations, including
their existing `LoggedIn`/`Inplace` metadata and thunks. No opcode
registration or scenario test body was moved. The character-family source scan
now includes the new child, and the ownership policy records all eleven moved
test/production method definitions under
`crate::handlers::character::items::handlers`.

The pinned TrinityCore source is
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`: the relevant target anchors are
`Handlers/ItemHandler.cpp::HandleSwapInvItemOpcode` (69–112),
`HandleAutoEquipItemSlotOpcode` (114–128), `HandleSwapItem` (130–173),
`HandleAutoEquipItemOpcode` (175+), `HandleAutoStoreBagItemOpcode` (699+),
and `HandleCancelTempEnchantmentOpcode` (1100+). These identify the adapters
only; the structural move makes no new parity claim.

Static source inventory after the move:

```text
wc -l crates/wow-world/src/handlers/character/items.rs crates/wow-world/src/handlers/character/items/handlers.rs
797 crates/wow-world/src/handlers/character/items.rs
516 crates/wow-world/src/handlers/character/items/handlers.rs
1313 total
```

The items root is below the ordinary 1,000-line review threshold. Destruction,
equipment-set, inventory-move, and login-load children remain separate.

No tests, builds, formatters, architecture checks, or QA were run for this
continuation under level 1. The dirty tree remains unvalidated; `HEAD` remains
the base SHA above, with no candidate SHA or commit.

#### Continuation: directory loot snapshot and delivery operations, level 1

The `PlayerRegistry` loot snapshot, recipient-resolution, loot-roll ownership,
and loot-money preparation methods moved into the private child
`session/directory/loot.rs`. The parent `session/directory.rs` remains the sole
owner of registry storage. Public method names and signatures, generation
checks, exact map/instance filters, canonical Player reads, command-channel
identity checks, and durable loot-money tracker selection are unchanged; no
packet registration, persistence fence, or publication order moved.

Current callers remain in the loot handlers and their claims, fanout, money,
roll, and request-context adapters, plus the existing Session money/group
consumers. The pinned TrinityCore revision
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd` provides the consumer context at
`Handlers/LootHandler.cpp::HandleAutostoreLootItemOpcode` (77–150) and
`HandleLootMoneyOpcode` (142+), `Loot/Loot.cpp::LootRoll::TryToStart`
(398–452) and `LootRoll::Finish` (575–620), `Entities/Player/Player.h::GetPassOnGroupLoot`
(2570), and `Entities/Player/Player.cpp::isAllowedToLoot` (17963–18006).
These anchors do not establish cross-language equivalence for the Rust
generation-checked directory projections; this organization-only move makes no
new gameplay-parity claim.

Static source inventory after the move:

```text
wc -l crates/wow-world/src/session/directory.rs crates/wow-world/src/session/directory/loot.rs
1574 crates/wow-world/src/session/directory.rs
268 crates/wow-world/src/session/directory/loot.rs
1842 total
```

The moved child stays within the usual cohesive-file range; the logical
`PlayerRegistry` owner and its storage do not change. No tests, builds,
formatters, architecture checks, or QA were run for this continuation under
level 1. The dirty tree remains unvalidated; `HEAD` remains the base SHA above,
with no candidate SHA or commit.

#### Continuation: directory recipient-query organization, level 1

The remaining read-only `PlayerRegistry` projections and recipient-candidate
queries moved into the private child `session/directory/recipient_queries.rs`.
This groups runtime, social, Group/Party, inspect, spatial, quest, vehicle,
aggro, and player-CREATE snapshots. Existing public method names and
signatures, registration generations, map/instance filters, canonical-owner
lookups, guard-drop order, and result ordering remain unchanged. The directory
struct and registration/lifetime operations remain in the parent. Non-loot
control-channel writes stay in their existing owner modules; loot-roll
ownership publication remains grouped with the loot-specific projections and
application preparation in `loot.rs`.

Consumers continue to call the same `PlayerRegistry` methods from chat, group,
inspect, quest, movement, spell, pet, collections, taxi, visibility, and
runtime publication code. The pinned TrinityCore revision
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd` provides related consumer anchors:
`Entities/Object/Object.cpp::WorldObject::SendMessageToSetInRange` (1752),
`Entities/Player/Player.cpp::Player::SendMessageToSetInRange` (6141),
`Maps/Map.cpp::Map::SendInitSelf` (1826),
`Handlers/InspectHandler.cpp::HandleInspectOpcode` (28), and
`Handlers/QuestHandler.cpp::HandleQuestConfirmAccept` (499). These references
frame the affected consumers only; the Rust directory snapshots and candidate
resolution are not asserted to be a complete C++-equivalent implementation.
No packet metadata, persistence, state transition, or publication behavior was
changed. `world_entry/login.rs` remains untouched because its physical-file
policy requires the explicit #584:C4 phase and reader/writer review before any
further split.

Static source inventory after both directory cuts:

```text
wc -l crates/wow-world/src/session/directory.rs crates/wow-world/src/session/directory/{loot.rs,recipient_queries.rs}
794 crates/wow-world/src/session/directory.rs
268 crates/wow-world/src/session/directory/loot.rs
788 crates/wow-world/src/session/directory/recipient_queries.rs
1850 total
```

The registry root and both children are below the ordinary 1,000-line review
threshold; this physical split does not reduce the logical `PlayerRegistry`
owner. No tests, builds, formatters, architecture checks, or QA were run for
this continuation under level 1. The dirty tree remains unvalidated; `HEAD`
remains the base SHA above, with no candidate SHA or commit.

#### Continuation: GameObject loot authority organization, level 1

The canonical GameObject loot-authority synchronization, personal-pool
installation observations and upserts, represented GameObject loot context,
fully-looted query, and autostore-distance decision moved into the private
`handlers/loot/sources/gameobject_authority.rs` child. The existing
`WorldSession` remains the sole owner; method names, signatures, state
transitions, cache reconciliation, generation/lifecycle checks, and callers are
preserved. Methods that previously had effective `crate::handlers::loot`
visibility retain that scope explicitly. Two helpers previously private to
`sources.rs` are `pub(super)` in the child so the parent and its descendant
modules can continue to call them. The session syntax-ownership inventory
records the new module paths and effective visibility. GameObject generation
and source orchestration remain in `sources.rs`; the two corpse lootable-flag
mutators also remain there.

The move was reviewed against the pinned TrinityCore revision
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`:
`Entities/GameObject/GameObject.cpp::GameObject::Use` (2501),
`SetLootState` (3683), `ClearLoot` (3711), and `GetLootForPlayer` (3898), plus
`Handlers/LootHandler.cpp::HandleAutostoreLootItemOpcode` (77). These anchors
establish source and consumer context only. No C++ behavior was ported or
changed in this slice, and no new gameplay-parity claim is made.

Static source inventory after this cut:

```text
wc -l crates/wow-world/src/handlers/loot/sources.rs crates/wow-world/src/handlers/loot/sources/gameobject_authority.rs crates/wow-world/src/handlers/loot/sources/gameobject.rs
966 crates/wow-world/src/handlers/loot/sources.rs
316 crates/wow-world/src/handlers/loot/sources/gameobject_authority.rs
996 crates/wow-world/src/handlers/loot/sources/gameobject.rs
2278 total
```

The source facade is below the ordinary 1,000-line review threshold, and the
existing GameObject operation child remains unchanged. This is a physical
organization change only; canonical loot ownership and lifecycle ordering do
not move. No tests, builds, formatters, architecture checks, or QA were run
under level 1. The dirty tree remains unvalidated; `HEAD` remains the base SHA
above, with no candidate SHA or commit.

#### Continuation: loot handler operation organization, level 1

The item-storage packet operation and its test adapter moved into the private
`handlers/loot/handlers/item.rs` child. Loot-money packet handling, its test
adapter, and the receiver-side apply/notification command methods moved into
`handlers/loot/handlers/money.rs`. The loot-release packet method moved beside
the existing open/close request operations in `handlers/loot/requests.rs`.
Method names, signatures, effective visibility, operation order, durable
boundaries, claim handling, generation checks, and fanout behavior are
unchanged. `session-ownership-policy.json` records the new implementation
modules.

Every `PacketHandlerEntry` for these opcodes remains in
`handlers/loot/handlers.rs`
with the same opcode, status, processing mode, name, and thunk. The pinned
TrinityCore revision
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd` supplies consumer context at
`Handlers/LootHandler.cpp::HandleAutostoreLootItemOpcode` (77),
`HandleLootMoneyOpcode` (142), and `HandleLootReleaseOpcode` (262). These
anchors are not evidence of a new port or gameplay equivalence; this change
only relocates the existing Rust operations.

Static source inventory after the cut:

```text
wc -l crates/wow-world/src/handlers/loot/handlers.rs crates/wow-world/src/handlers/loot/handlers/{item.rs,money.rs} crates/wow-world/src/handlers/loot/requests.rs
987 crates/wow-world/src/handlers/loot/handlers.rs
286 crates/wow-world/src/handlers/loot/handlers/item.rs
534 crates/wow-world/src/handlers/loot/handlers/money.rs
365 crates/wow-world/src/handlers/loot/requests.rs
2172 total
```

The packet-registration facade is below the ordinary 1,000-line review
threshold, and both operation children remain within the preferred cohesive
file range. No tests, builds, formatters, architecture checks, or QA were run
under level 1. The dirty tree remains unvalidated; `HEAD` remains the base SHA
above, with no candidate SHA or commit.

#### Continuation: loot release transition organization, level 1

The source-specific release transition, active-view close-out, and the
GameObject/gathering-node release publication helpers moved from
`handlers/loot/claims.rs` into the private child
`handlers/loot/claims/release.rs`. Canonical loot authority, object and view
generation checks, claim rollback/commit ordering, cache reconciliation,
durable release effects, and packet/fanout order remain unchanged. Existing
callers in packet requests, loot persistence, Session lifecycle, spell/item
operations, GameObject handling, and the loot test module retain the same
`WorldSession` method names and signatures. The methods formerly `pub(super)`
now explicitly use `pub(in crate::handlers::loot)` to preserve their effective
scope; the all-views release entry point remains `pub(crate)`. The syntax
ownership inventory records the child module and visibility.

The move was reviewed against pinned TrinityCore revision
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`:
`Handlers/LootHandler.cpp::HandleLootReleaseOpcode` (262) and
`WorldSession::DoLootRelease` (270),
`Entities/GameObject/GameObject.cpp::OnLootRelease` (3735), and
`Entities/Creature/Creature.cpp::AllLootRemovedFromCorpse` (2942). These
anchors establish source and lifecycle context only; this structural cut makes
no new gameplay-parity claim.

Static source inventory after the cut:

```text
wc -l crates/wow-world/src/handlers/loot/claims.rs crates/wow-world/src/handlers/loot/claims/release.rs
559 crates/wow-world/src/handlers/loot/claims.rs
690 crates/wow-world/src/handlers/loot/claims/release.rs
1249 total
```

Both files remain below the preferred 800-line cohesive-file target. No tests,
builds, formatters, architecture checks, or QA were run under level 1. The
dirty tree remains unvalidated; `HEAD` remains the base SHA above, with no
candidate SHA or commit.

#### Continuation: loot-roll criteria and publication organization, level 1

Loot-roll criterion updates and the packet-send/broadcast helpers moved from
`handlers/loot/rolls.rs` into the private child
`handlers/loot/rolls/publication.rs`. The roll-state owner, voting and winner
selection, RNG and timeout processing, claim lifecycle, recipient resolution,
packet values, and call order remain in their existing implementation. The
parent calls the extracted helpers with their former effective private scope:
they are `pub(super)` in the child, visible only within the `rolls` module and
its descendants. `session-ownership-policy.json` records the source modules
and visibility.

The move was reviewed against the pinned TrinityCore revision
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd` at
`Loot/Loot.cpp::LootRoll::TryToStart` (398), `PlayerVote` (454),
`UpdateRoll` (498), `AllPlayerVoted` (520), and `Finish` (575). These anchors
describe the roll lifecycle context only; the relocation does not port or
change roll behavior and makes no new parity claim.

Static source inventory after the cut:

```text
wc -l crates/wow-world/src/handlers/loot/rolls.rs crates/wow-world/src/handlers/loot/rolls/publication.rs
943 crates/wow-world/src/handlers/loot/rolls.rs
265 crates/wow-world/src/handlers/loot/rolls/publication.rs
1208 total
```

The roll owner is below the ordinary 1,000-line review threshold and the
publication child remains within the preferred cohesive-file range. No tests,
builds, formatters, architecture checks, or QA were run under level 1. The
dirty tree remains unvalidated; `HEAD` remains the base SHA above, with no
candidate SHA or commit.

#### Continuation: shared character-handler support organization, level 1

Four cohesive helper groups moved out of `handlers/character/mod.rs` into
private child modules: login location/homebind and initial world-state support
in `login_support.rs`; persisted map-transport validation, position and
create-block composition in `login_transport_support.rs`; character-list flag
and pet projections in `enumeration_support.rs`; and race/class creation
defaults and restored-health support in `creation_support.rs`. Their
implementations, callers, and operation order were relocated without changing
the `WorldSession` owner or behavior. Parent imports preserve descendant
call-sites. `default_display_id` remains available at its existing
`crate::handlers::character` path, and the player-visibility snapshot helper
retains its crate-level path through an explicit re-export.

The source review used pinned TrinityCore revision
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`:
`Server/Packets/CharacterPackets.cpp` (118-156),
`Entities/Player/Player.cpp::Player::Create` (386),
`Player::LoadFromDB` (17060), `Player::_LoadHomeBind` (19228), and
`Player::SendInitialPacketsBeforeAddToMap` (23455),
`Maps/Map.cpp::Map::SendInitTransports` (1853), and
`World/WorldStates/WorldStateMgr.cpp::FillInitialWorldStates` (259). These
anchors establish operation and caller context only; no Rust logic was
changed, and the structural move makes no new gameplay-parity claim.

Static source inventory after the cut:

```text
wc -l crates/wow-world/src/handlers/character/mod.rs crates/wow-world/src/handlers/character/{creation_support.rs,enumeration_support.rs,login_support.rs,login_transport_support.rs}
978 crates/wow-world/src/handlers/character/mod.rs
105 crates/wow-world/src/handlers/character/creation_support.rs
92 crates/wow-world/src/handlers/character/enumeration_support.rs
274 crates/wow-world/src/handlers/character/login_support.rs
406 crates/wow-world/src/handlers/character/login_transport_support.rs
1855 total
```

The shared character-handler facade is below the ordinary 1,000-line review
threshold, and each extracted support module is below 800 lines. The existing
character-family publication-order source scan now includes all four new
files. No tests, builds, formatters, architecture checks, or QA were run under
level 1. The worktree remains unvalidated; `HEAD` remains
`9daa13f663bd1e863a3efed06721c3fcb3b6cd66`, with no candidate SHA or commit.

#### Continuation: lifecycle-persistence test organization and Session-state review, level 1

The shared `RecordingPortLikeCpp` fixture and session builders remain in
`session/tests/lifecycle_persistence.rs`. Its 34 persistence scenarios moved
without test-body or attribute changes into `player_persistence.rs` (14),
`character_lifecycle.rs` (14), and `account_collections.rs` (6). The existing
`deferred_transfer.rs` and `save_interleaving.rs` children remain registered;
the parent module path in `session_tests.rs` is unchanged. This is test-source
organization only; no production persistence code or contract changed.

Static inventory after the cut:

```text
wc -l crates/wow-world/src/session/tests/lifecycle_persistence.rs crates/wow-world/src/session/tests/lifecycle_persistence/{account_collections.rs,character_lifecycle.rs,player_persistence.rs}
413 crates/wow-world/src/session/tests/lifecycle_persistence.rs
111 crates/wow-world/src/session/tests/lifecycle_persistence/account_collections.rs
372 crates/wow-world/src/session/tests/lifecycle_persistence/character_lifecycle.rs
432 crates/wow-world/src/session/tests/lifecycle_persistence/player_persistence.rs
1328 total
```

The current unvalidated worktree inventory also supersedes the earlier
Session-root measurements in this section: `session/mod.rs` is 1,027 lines,
`session/state.rs` is 2,090, and `session/construction.rs` is 1,185. The
2,090-line `state.rs` is the single `WorldSession` definition and its field
list; this pass did not alter it. Splitting fields into a generic state bag
would obscure responsibility, while moving a partial field group without all
direct readers would break the existing internal access paths. No complete
semantic field-group migration is established here. This is a pending
macro-closeout review, not a terminal exception: #1233 acceptance must either
complete a responsibility-based migration with one canonical Session state
or record a specific, evidence-backed bounded exception under the module
policy. The 1,185-line constructor remains an ordinary cohesion-review item.

No tests, builds, formatters, architecture checks, or QA were run under level
1. The worktree remains unvalidated at base `9daa13f663bd1e863a3efed06721c3fcb3b6cd66`,
with no candidate SHA or commit.

#### Continuation: world-entity scenario test organization, level 1

The 1,993-line `session/tests/scenarios_world_entities_19.rs` scenario module
is now a 22-line registration facade with seven focused children: GameObject
use (2 tests), shared creature authority (2), creature tick ownership (4),
creature movement ticks (3), legacy melee/aggro no-op behavior (2), player
melee modifiers (3), and player melee outcomes (4). The existing registration
path in `session_tests.rs` is unchanged, and each child retains its parent
session-test fixture access.

Static source comparison against `HEAD` matched all 20 test attributes and
bodies, with no duplicate or missing registrations. Existing test comments
remain with their corresponding scenarios. The largest resulting child is
`player_melee_outcomes.rs` at 600 lines; all seven children are below the
800-line preferred cohesive-file ceiling. This is test organization only: no
production behavior, test assertion, or parity claim changed.

No tests, builds, formatters, architecture checks, or QA were run under level
1. The worktree remains unvalidated at base `9daa13f663bd1e863a3efed06721c3fcb3b6cd66`,
with no candidate SHA or commit.

#### Continuation: player-victim creature-melee scenario organization, level 1

The current 1,954-line `session/tests/scenarios_world_entities_32.rs` suite is
now a 15-line facade with three children: hit-band and armor (2 tests,
`melee_hit_and_armor.rs`, 612 lines), damage modifiers and expected-stat use
(3 tests, `melee_damage_modifiers.rs`, 569 lines), and absorption/mana-shield
behavior (3 tests, `melee_absorption.rs`, 781 lines). The registration path
through the existing session test module is unchanged. The two ownership-ledger
entries for armor and victim damage-taken evidence now name their actual nested
test modules; their test fingerprints remain unchanged.

Static comparison against the current worktree source preserved all eight
remaining test attributes, bodies, and leading C++ evidence comments. The
pre-existing `player_block_percent_matches_get_block_percent_like_cpp` case
remains in `wow-combat/tests/melee_contract.rs`; this slice adds no test
removals. This is test organization only and makes no new gameplay or parity
claim.

No tests, builds, formatters, architecture checks, or QA were run under level
1. The worktree remains unvalidated at base `9daa13f663bd1e863a3efed06721c3fcb3b6cd66`,
with no candidate SHA or commit.

#### Continuation: creature-melee absorb-stage organization, level 1

The canonical shield-resolution block moved from
`session/legacy_runtime/creature_melee_tick.rs` into the private
`creature_melee_tick/absorption.rs` child. It contains the existing school- and
mana-shield operations for canonical player and creature victims, plus the
shield-amount writer. The callers remain in the same melee coordinator; only
the two helpers called by that parent are `pub(super)`, retaining the former
effective scope. The external `run_legacy_creature_melee_tick_once_like_cpp`
path and map-owned mutation/publication ordering are unchanged.

The source review used pinned TrinityCore revision
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`, `Entities/Unit/Unit.cpp`,
`Unit::CalcAbsorbResist` (1789-1930), whose school-absorb loop precedes its
mana-shield loop. Static comparison found the parent identical outside the
extracted block, and the moved block identical apart from those two visibility
qualifiers. The parent is now 1,608 lines and the child 269 lines. This is a
physical organization change only; it changes no combat formula or parity
claim.

No tests, builds, formatters, architecture checks, or QA were run under level
1. The worktree remains unvalidated at base `9daa13f663bd1e863a3efed06721c3fcb3b6cd66`,
with no candidate SHA or commit.

#### Continuation: legacy creature-melee scenario organization, level 1

The 1,669-line `session/tests/scenarios_world_entities_28.rs` suite is now a
12-line facade with two focused children: `creature_melee_admission.rs` (8
scenarios, 685 lines) and `creature_melee_damage.rs` (5 scenarios, 988 lines).
The `session_tests.rs` registration path remains unchanged; no local helpers or
include-based consumers needed relocation.

Static comparison against the pre-move source preserved all 13 test attributes,
bodies, and leading C++ comments without duplicate or missing registrations.
The destination scenario module has no session-ownership-ledger entries to
retarget. This is a test-organization change only; no production behavior or
test assertion changed.

No tests, builds, formatters, architecture checks, or QA were run under level
1. The worktree remains unvalidated at base `9daa13f663bd1e863a3efed06721c3fcb3b6cd66`,
with no candidate SHA or commit.

#### Continuation: player-item and combat-stat scenario organization, level 1

The current 1,961-line `session/tests/scenarios_player_items_12.rs` suite is
now a 16-line facade with four responsibility children: item/equipment updates
(5 tests, 354 lines), defensive combat stats (3, 369), weapon offense (5, 697),
and resistance/spell power (5, 555). The `session_tests.rs` mount is unchanged,
and there are no exact module-path entries in the ownership ledger to retarget.

Static comparison against the pre-move worktree source preserved all 18 test
attributes, bodies, and leading comments. It also retains the pre-existing
canonical item-push import-path adjustment in its original test. No test
assertion or production behavior changed in this organization pass.

No tests, builds, formatters, architecture checks, or QA were run under level
1. The worktree remains unvalidated at base `9daa13f663bd1e863a3efed06721c3fcb3b6cd66`,
with no candidate SHA or commit.

#### Continuation: spell-effect scenario organization, level 1

The 1,905-line `session/tests/scenarios_spell_state_12.rs` suite is now a
module root retaining the positivity-classification scenario plus five focused
children: player power effects (13 tests, 767 lines), extra attacks (3, 208),
inebriate effects (4, 196), reputation (4, 257), and creature power effects
(4, 462). Its `session_tests.rs` mount is unchanged. The pre-existing change to
exercise the positivity rule through `wow-data` remains in the root test.

Static comparison against the current pre-move source matched all 29 test
attributes, bodies, and leading comments, with no missing or duplicated cases.
This is test organization only; no production behavior, assertion, or parity
claim changed.

No tests, builds, formatters, architecture checks, or QA were run under level
1. The worktree remains unvalidated at base `9daa13f663bd1e863a3efed06721c3fcb3b6cd66`,
with no candidate SHA or commit.

#### Continuation: spell-state and direct-effect scenario organization, level 1

The 1,854-line `session/tests/scenarios_spell_state_11.rs` suite is now a
35-line root retaining the replacement-spell fixture plus five children:
spell-state ownership/registry (6 tests, 611 lines), reputation (2, 91), base
damage (4, 265), healing (4, 356), and damage modifiers (4, 534). The existing
`session_tests.rs` mount is unchanged.

Static comparison against the pre-move source preserved all 20 test attributes,
bodies, and leading comments; the shared fixture is present once at the parent
and remains accessible to its child. No ownership-policy module entries needed
retargeting. This is test organization only; no production behavior or assertion
changed.

No tests, builds, formatters, architecture checks, or QA were run under level
1. The worktree remains unvalidated at base `9daa13f663bd1e863a3efed06721c3fcb3b6cd66`,
with no candidate SHA or commit.

#### Continuation: additional Session scenario organization, level 1

Thirteen further oversized scenario roots were decomposed into responsibility-
scoped child modules. Together with the previously documented
`scenarios_spell_state_11.rs` split, this brings the current inventory to
fourteen roots. All `session_tests.rs` registration mounts remain unchanged.

| Existing root | New children (tests; lines) |
| --- | --- |
| `scenarios_combat_4.rs` (1,674 → 11 lines; 14 tests) | `white_swing_damage` (4; 383), `attack_table_outcomes` (3; 408), `melee_damage_taken` (3; 327), `avoidance_critical_and_evade` (4; 557) |
| `scenarios_player_items_5.rs` (1,441 → 11; 17 tests) | `appearance_admission_and_updates` (7; 365), `can_add_appearance_gates` (5; 738), `quest_reward_appearances` (3; 308), `appearance_queries` (2; 31) |
| `scenarios_spell_state_25.rs` (1,437 → 157; 19 tests) | `health_derived_aurastate` (3; 199), `spell_damage_and_healing` (5; 363), `spell_power_coefficients` (3; 192), `attack_speed_and_form_timing` (2; 159), `shapeshift_forms_and_display_power` (6; 378) |
| `scenarios_combat_1.rs` (1,370 → 13; 18 tests) | `combat_rating_and_swing_bonuses` (4; 522), `quest_kill_rewards` (2; 129), `combat_reach_and_visibility` (2; 90), `canonical_player_ownership` (2; 177), `death_and_healing` (3; 94), `combat_tick_admission` (5; 372) |
| `scenarios_world_entities_10.rs` (1,161 → 12; 14 tests) | `creature_kill_and_death` (3; 223), `spell_damage_death` (2; 102), `spell_healing` (3; 192), `spell_threat` (4; 364), `creature_aura_application` (2; 290) |
| `scenarios_player_items_9.rs` (1,130 → 10; 20 tests) | `durability_scenarios` (5; 626, with 3 fixtures), `item_template_rules` (4; 338), `inventory_position_and_open_item` (11; 178) |
| `scenarios_world_entities_35.rs` (1,106 → 8; 4 tests) | `shared_damage_scenarios` (3; 836, with 3 fixtures), `unkillable_scenario` (1; 269); nested `share_damage` mount in `_34` is unchanged |
| `scenarios_spell_state_26.rs` (1,062 → 19; 10 tests) | `power_drain_core` (2; 225), `power_drain_damage_taken` (7; 764), `creature_negative_damage_taken_aura` (1; 69) |
| `scenarios_spell_state_16.rs` (1,010 → 17; 14 tests) | `healing_and_health_leech` (6; 376), `kill_credit` (2; 169), `honor_effects` (3; 191), `pet_dismissal` (2; 114), `heal_absorb` (1; 171) |
| `scenarios_misc_3.rs` (1,026 → 15; 13 tests) | `map_value_publication` (3; 174), `dynamic_object_snapshots` (5; 288), `rest_and_far_sight` (4; 214), `trainer_interaction` (1; 357) |
| `login_auxiliary_persistence.rs` (1,055 → 311; 12 tests) | `auxiliary_login_reads` (4; 180), `spell_history` (2; 114), `trait_configuration` (6; 460); the typed persistence-port fixture remains at the parent |
| `scenarios_world_entities_1.rs` (1,048 → 17; 19 tests) | `combat_catalog_and_fanout` (4; 119), `creature_attack_commands` (4; 221), `creature_melee_commands` (6; 419), `durable_runtime_rail` (2; 106), `visibility_refresh_commands` (3; 200) |
| `player_spell_hit_source.rs` (1,023 → 191; 18 tests) | `identity_and_lifetime` (4; 184), `trait_glyph_and_zone_gates` (2; 122), `outdoor_pvp_and_area_ancestry` (4; 131), `pet_and_login_sources` (5; 213), `spell_area_requirements` (2; 103), `source_mutation_invalidation` (1; 98); three shared authority fixtures remain at the parent |

The shared fixture extraction leaves `session_tests.rs` as a 389-line
registration root and `session/tests/fixtures/mod.rs` as a 54-line private
aggregator. Fifteen responsibility-named fixture modules remain there; the
largest is `spell_catalog.rs` at 705 lines. Static comparisons preserved every
test attribute and body in the fourteen scenario roots, their shared fixture
functions, and attached C++ comments. The pre-existing
`wow_core::visibility_distance_allows_like_cpp` path change was retained; no
ownership-policy module paths needed retargeting. These are test-organization
changes only.

A scoped physical count using
`rg --files crates/wow-world/src/session/tests | rg '\.rs$' | xargs wc -l | sort -nr`
reported 124,310 total lines and no Rust test file above 1,000 lines; the largest
remaining file is `scenarios_world_entities_28/creature_melee_damage.rs` at
988. This is navigation evidence for the test directory, not architecture
acceptance.

#### Continuation: spell-effect application modules, level 1

`session/spell_effects/effect_combat.rs` is a 691-line parent retaining the
shared damage, healing, and target projections. The existing
`session/spell_effects/mod.rs` mount remains unchanged. Damage/environmental
and taunt operations live in
`effect_combat/damage_and_combat_application.rs` (516 lines); heal
application and publication live in
`effect_combat/healing_application.rs` (384 lines).

Static comparison against the base plus the pre-existing local call-path
adjustment preserved all 27 `WorldSession` method signatures, visibilities,
bodies, and attached method documentation exactly. The existing
`crate::session::player_aura_info_like_cpp` call path remains in the moved heal
method. No caller or gameplay behavior changed.

#### Continuation: receiver-free Session rules, level 1

`session_rules/rules_3.rs` is now a 14-line facade over two responsibility-
scoped children: aura projections in `rules_3/aura_effects.rs` (558 lines) and
melee-damage calculations in `rules_3/melee_damage.rs` (283 lines).
`session_rules/mod.rs` re-exports preserve the prior crate-internal paths.

Static comparison preserved all 25 top-level item blocks, including attached
documentation: 19 aura-projection items and 6 melee-damage items. Only module
placement and imports changed; callers and gameplay behavior are unchanged.

#### Continuation: character-persistence test organization, level 1

The 1,739-line `handlers/character_tests/persistence.rs` now retains 17
login/persistence scenarios in a 783-line parent. The mana, health and power
regeneration scenarios, food/drink visual scenarios, and their 10 local helpers
now live under `persistence/resource_regeneration/`: the 31-line root retains
the shared power-type fixture, `mana_and_food_emotes.rs` contains 7 tests
(484 lines), and `health_and_power_regeneration.rs` contains 6 tests (473
lines). The existing `character_tests.rs` mount remains unchanged.

Static comparison against the current pre-move worktree source preserved all 23
moved top-level function blocks, including test attributes, bodies, helpers and
attached comments. No block was lost or duplicated. The pre-existing
`game_time_ms_like_cpp` path adjustments remain in the moved scenarios; the
existing login-source path adjustment remains in the parent. This is test
organization only; assertions and production behavior are unchanged.

#### Continuation: Session aura authority and query organization, level 1

`session/spell_state/aura.rs` is now a 737-line parent with two focused
children: `aura/spell_hit_authority.rs` (13 methods; 291 lines) contains the
canonical spell-hit aura-source proof and synchronization operations, while
`aura/effect_queries.rs` (21 methods; 366 lines) contains aura-effect
queries and modifier projections. The existing `spell_state/mod.rs` mount
remains unchanged. The parent retains 18 methods for aura-state projection,
shapeshift, loading, and transform operations, together with their local
helpers.

Static comparison preserved all 52 `WorldSession` method blocks, including
signatures, visibility, attributes, bodies, and attached documentation. Existing
`pub(in crate::session)` and `pub(crate)` method visibility and call sites
remain unchanged. This is a physical organization change only.

#### Continuation: movement spline progression organization, level 1

`session/movement/state.rs` is now a 955-line parent with spline/taxi
progression operations in the private `state/spline_progression.rs` child
(235 lines). The child contains five complete methods: move-time-skipped
application, spline completion, taxi-spline completion, its private event
recording helper, and the taxi-event projection. The existing
`session/movement/mod.rs` mount remains unchanged.

Static comparison preserved all 56 `WorldSession` method blocks, including
signatures, visibility, attributes, bodies, and attached documentation. The
private taxi-event helper moved with its callers; all existing external method
call paths remain unchanged. This is a physical organization change only.

#### Continuation: character-visibility handler organization, level 1

`handlers/character/visibility.rs` is now a 352-line parent retaining
creature-spawn materialization and registration. Nearby-creature delivery and
visibility refresh live in `visibility/creatures.rs` (5 methods; 970 lines);
nearby GameObject delivery lives in `visibility/gameobjects.rs` (1 method;
244 lines). The existing `handlers/character/mod.rs` mount remains unchanged.

Static comparison preserved all 10 `WorldSession` method blocks, including
signatures, visibilities, bodies, and attached documentation. The private
creature-create helper moved with its consumers; the parent materialization
helpers and their existing effective visibility remain available to the nested
modules. No packet registrations or call sites changed. This is a physical
organization change only.

No tests, builds, formatters, architecture checks, or QA were run for these
continuations under level 1. The worktree remains unvalidated at base
`9daa13f663bd1e863a3efed06721c3fcb3b6cd66`, with no candidate SHA or commit.

#### Continuation: loot random-property organization, level 1

The random-property representation, stack-compatibility helper, weighted
enchantment selection, and `WorldSession` random-property generation methods
moved from `handlers/loot/mod.rs` to the private
`handlers/loot/random_properties.rs` module. Existing callers in creature loot
generation and item/disenchant storage retain their method and helper names.
The new child uses `pub(super)` only where needed to preserve the former
visibility throughout the `handlers::loot` subtree; the helper implementations
and caller-supplied RNG remain unchanged. The ownership syntax policy records
the moved `WorldSession` methods under the new module.

The structural move was reviewed against pinned TrinityCore 3.4.3 source
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`: `ItemEnchantmentMgr.cpp::GetRandomPropertyPoints`
(81) and `ItemEnchantmentMgr::GenerateRandomProperties` (153), plus
`Player.cpp::StoreLootItem` (25643), which invokes that generator during item
storage. These anchors establish operation context only; this change makes no
new parity claim. Inventory mutation, loot claims, persistence, packet
publication, and opcode registrations are unchanged.

The weighted selector remains in this private `wow-world` adapter. It consumes
`ItemRandomEnchantmentTemplateEntry` from `wow-data`, which is categorized as
`adapter-platform`, while `wow-loot` is `domain-runtime` under
`dependency-policy.json`; adding that edge would invert the allowed dependency
direction. A generic row projection solely for this one selector would add an
unearned adapter, so no new crate or dependency was introduced.

Static source inventory after the cut:

```text
wc -l crates/wow-world/src/handlers/loot/mod.rs crates/wow-world/src/handlers/loot/random_properties.rs
1043 crates/wow-world/src/handlers/loot/mod.rs
170 crates/wow-world/src/handlers/loot/random_properties.rs
1213 total
```

No tests, builds, formatters, architecture checks, or QA were run for this
continuation under level 1. The worktree remains unvalidated at the base SHA,
with no candidate SHA or commit.

#### Continuation: loot item-storage plan organization, level 1

The shared direct-loot/disenchant plan records and the post-store publication
context moved from `handlers/loot/mod.rs` into the private
`handlers/loot/storage_plans.rs` module. Their fields remain available only
within the `handlers::loot` subtree. Existing call sites in item storage,
disenchant, claim, persistence, and handler operations retain the same type
names. Planning, transaction/claim ordering, canonical item application, and
publication remain in their existing operation owners.

The C++ operation context is `Player::StoreLootItem` in the pinned
TrinityCore 3.4.3 source `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`
(`Entities/Player/Player.cpp:25643-25710`). This relocation does not change
that sequence or claim a new parity result.

Static source inventory after this cut:

```text
wc -l crates/wow-world/src/handlers/loot/mod.rs crates/wow-world/src/handlers/loot/random_properties.rs crates/wow-world/src/handlers/loot/storage_plans.rs
978 crates/wow-world/src/handlers/loot/mod.rs
170 crates/wow-world/src/handlers/loot/random_properties.rs
79 crates/wow-world/src/handlers/loot/storage_plans.rs
1227 total
```

The loot facade is now below the ordinary 1,000-line review threshold;
remaining responsibility and ownership work stays in #1233. No tests, builds,
formatters, architecture checks, or QA were run under level 1. The worktree
remains unvalidated at the base SHA, with no candidate SHA or commit.

#### Continuation: canonical loot test-fixture organization, level 1

The shared builders and snapshots for canonical map objects moved from
`handlers/loot_tests.rs` into the private `loot_tests/canonical_world.rs`
fixture module. It groups the existing WorldObject, Creature, GameObject, and
Corpse setup/lookup helpers. All 12 helper names remain available through
explicit imports in the parent fixture module, so existing creature,
GameObject, corpse, login, and miscellaneous scenarios retain their calls and
registrations. No test scenario or production operation moved or changed.

Static source inventory after the cut:

```text
wc -l crates/wow-world/src/handlers/loot_tests.rs crates/wow-world/src/handlers/loot_tests/canonical_world.rs
1609 crates/wow-world/src/handlers/loot_tests.rs
190 crates/wow-world/src/handlers/loot_tests/canonical_world.rs
1799 total
```

The shared test facade remains above 1,000 lines and still needs further
fixture decomposition within #1233. No tests, builds, formatters,
architecture checks, or QA were run under level 1. The worktree remains
unvalidated at the base SHA, with no candidate SHA or commit.

#### Continuation: shared loot-authority test fixtures, level 1

The reusable coin-loot, canonical loot-authority, two-session snapshot,
response, and disenchant-output fixtures moved from `handlers/loot_tests.rs`
into `loot_tests/loot_authority.rs`. The six helper names remain imported into
the parent fixture module, preserving existing calls from creature, item,
GameObject, login, quest, spell, persistence, and miscellaneous scenarios.
The cached-authority helper still rebuilds the same represented personal-loot
counters before canonical synchronization; only its internal import path now
names the existing parent `handlers::loot` helper directly. No fixture data,
test scenario, registration, or production operation changed.

Static source inventory after the cut:

```text
wc -l crates/wow-world/src/handlers/loot_tests.rs crates/wow-world/src/handlers/loot_tests/canonical_world.rs crates/wow-world/src/handlers/loot_tests/loot_authority.rs
1424 crates/wow-world/src/handlers/loot_tests.rs
190 crates/wow-world/src/handlers/loot_tests/canonical_world.rs
211 crates/wow-world/src/handlers/loot_tests/loot_authority.rs
1825 total
```

The shared fixture facade remains above 1,000 lines and needs further
responsibility-scoped decomposition within #1233. No tests, builds,
formatters, architecture checks, or QA were run under level 1. The worktree
remains unvalidated at the base SHA, with no candidate SHA or commit.

#### Continuation: shared loot item-template fixtures, level 1

The limited-item, disenchantable-item, and random-property item-record builders
moved from `handlers/loot_tests.rs` into the private
`loot_tests/item_templates.rs` module. The five fixture names remain explicitly
imported by the parent, so existing item, loot, creature, GameObject, login,
persistence, and miscellaneous scenarios keep their calls. The builders,
template values, and scenario registrations are unchanged; this is test-fixture
organization only.

Static source inventory after this cut:

```text
wc -l crates/wow-world/src/handlers/loot_tests.rs crates/wow-world/src/handlers/loot_tests/item_templates.rs
1277 crates/wow-world/src/handlers/loot_tests.rs
171 crates/wow-world/src/handlers/loot_tests/item_templates.rs
1448 total
```

The shared test facade remains above 1,000 lines and needs further
responsibility-scoped decomposition within #1233. No tests, builds,
formatters, architecture checks, or QA were run under level 1. The worktree
remains unvalidated at the base SHA, with no candidate SHA or commit.

#### Continuation: group-loot lifecycle test fixtures, level 1

The shared group/master-loot setup and generation-tagged group-roll fixtures
moved from `handlers/loot_tests.rs` into the private
`loot_tests/group_lifecycle.rs` module. The five helper names remain explicitly
imported by the parent, preserving their existing uses in creature, item,
GameObject, spell, and loot-roll scenarios. The fixture values, assertions,
session setup, and registrations are unchanged; production loot ownership and
runtime behavior are untouched.

Static source inventory after this cut:

```text
wc -l crates/wow-world/src/handlers/loot_tests.rs crates/wow-world/src/handlers/loot_tests/group_lifecycle.rs
1130 crates/wow-world/src/handlers/loot_tests.rs
168 crates/wow-world/src/handlers/loot_tests/group_lifecycle.rs
1298 total
```

The shared test facade remains above 1,000 lines and needs further
responsibility-scoped decomposition within #1233. No tests, builds,
formatters, architecture checks, or QA were run under level 1. The worktree
remains unvalidated at the base SHA, with no candidate SHA or commit.

#### Continuation: overworld personal-loot test fixtures, level 1

The shared overworld personal-loot scenario builder, fixture record, generation
assertions, and independent-claim assertions moved into the private
`loot_tests/overworld_personal_loot.rs` module. The parent explicitly imports
the three existing helper functions, preserving consumers in creature and
miscellaneous scenarios. The fixture values and assertions are unchanged; no
production generation, authority, claim, or publication path moved.

Static source inventory after this cut:

```text
wc -l crates/wow-world/src/handlers/loot_tests.rs crates/wow-world/src/handlers/loot_tests/overworld_personal_loot.rs
879 crates/wow-world/src/handlers/loot_tests.rs
275 crates/wow-world/src/handlers/loot_tests/overworld_personal_loot.rs
1154 total
```

The shared loot test facade is now below the ordinary 1,000-line review
threshold; other large scenario and fixture modules remain in scope for #1233.
No tests, builds, formatters, architecture checks, or QA were run under level 1.
The worktree remains unvalidated at the base SHA, with no candidate SHA or
commit.

#### Continuation: quest source-item test fixtures, level 1

Quest source-item template, item-record, limit-category, and direct-inventory
builders moved from `handlers/quest_tests.rs` into the private
`handlers/quest_tests/source_items.rs` module. The parent fixture module exposes
only the helpers used by its existing child scenarios; those scenarios retain
their calls and registrations. The helper bodies and fixture values were moved
without modification. Production quest operations and their owners remain
unchanged; the related source path remains
`Player.cpp::GiveQuestSourceItem` at the pinned TrinityCore revision documented
above. No new gameplay-parity claim is made.

Static source inventory after this cut:

```text
wc -l crates/wow-world/src/handlers/quest_tests.rs crates/wow-world/src/handlers/quest_tests/source_items.rs
1187 crates/wow-world/src/handlers/quest_tests.rs
236 crates/wow-world/src/handlers/quest_tests/source_items.rs
1423 total
```

The shared quest-test fixture facade remains above the ordinary 1,000-line
review threshold and needs further responsibility-scoped decomposition within
#1233. No tests, builds, formatters, architecture checks, or QA were run under
level 1. The worktree remains unvalidated at base
`9daa13f663bd1e863a3efed06721c3fcb3b6cd66`, with no candidate SHA or commit.

#### Continuation: quest-party and catalog-persistence fixtures, level 1

The shared quest-test facade's represented party/group setup moved into
handlers/quest_tests/party.rs; quest-POI and item-template-addon persistence
ports moved into handlers/quest_tests/catalog_persistence.rs. Existing child
scenarios continue to use the same fixture names through explicit parent
imports. Canonical Player setup remains test-only and still writes through the
canonical map; no production state owner, persistence boundary, or quest-sharing
operation changed. These moves add no gameplay-parity claim.

Static source inventory after these cuts:

    wc -l crates/wow-world/src/handlers/quest_tests.rs crates/wow-world/src/handlers/quest_tests/party.rs crates/wow-world/src/handlers/quest_tests/catalog_persistence.rs
    973 crates/wow-world/src/handlers/quest_tests.rs
    161 crates/wow-world/src/handlers/quest_tests/party.rs
    83 crates/wow-world/src/handlers/quest_tests/catalog_persistence.rs
    1217 total

The shared quest-test facade is now below the ordinary 1,000-line review
threshold. No tests, builds, formatters, architecture checks, or QA were run
under level 1. The worktree remains unvalidated at base
9daa13f663bd1e863a3efed06721c3fcb3b6cd66, with no candidate SHA or commit.

#### Continuation: spell-learning test organization, level 1

The 17 tests formerly held together in session/effect_learning_tests.rs are now
grouped under three focused children: base_fallback.rs, spell_effects.rs, and
spell_fallback.rs. The original test bodies, assertions, fixtures, and
production calls were relocated without behavior changes; only the Rust test
module paths changed. No production code or canonical owner moved, and no new
spell-parity claim is made.

Static source inventory after this cut:

    wc -l crates/wow-world/src/session/effect_learning_tests.rs crates/wow-world/src/session/effect_learning_tests/base_fallback.rs crates/wow-world/src/session/effect_learning_tests/spell_effects.rs crates/wow-world/src/session/effect_learning_tests/spell_fallback.rs
    7 crates/wow-world/src/session/effect_learning_tests.rs
    409 crates/wow-world/src/session/effect_learning_tests/base_fallback.rs
    522 crates/wow-world/src/session/effect_learning_tests/spell_effects.rs
    92 crates/wow-world/src/session/effect_learning_tests/spell_fallback.rs
    1030 total

All three child files remain within the ordinary 1,000-line review threshold.
No tests, builds, formatters, architecture checks, or QA were run under level 1.
The worktree remains unvalidated at base
9daa13f663bd1e863a3efed06721c3fcb3b6cd66, with no candidate SHA or commit.

## P2 Player base-stat catalog classification — candidate, 2026-09-14

El siguiente C0–C4 audit clasifica `player_stats` como un catálogo inmutable de
configuración/servicios, no como autoridad de estado de `WorldSession`. TrinityCore
mantiene las filas de raza/clase/nivel en `ObjectMgr::_playerInfo` y expone
`ObjectMgr::GetPlayerLevelInfo` (`Globals/ObjectMgr.h:628-673,1155,1866`,
`ObjectMgr.cpp:4429-4445`); `Player` solo consulta esa autoridad durante el cálculo
de sus estadísticas (`Player.cpp:2256-2260,2370-2374`). Rust ya carga el catálogo una
vez en `world-server/app.rs:2222-2240`, lo compone en la capacidad `SessionInventoryCapabilitiesLikeCpp`
(`session_resources.rs:65,107`) y lo conserva como `Arc<PlayerStatsStore>` para los
lectores de estadísticas (`handlers/character/stats.rs:117`, `session_state.rs:1103`).
El ledger registra la clasificación en `immutable_catalogs_configuration_and_services`;
no se mueve gameplay, no se crea una autoridad duplicada y el residual exacto queda en
6 campos hasta la integración.

## P3 Creature query duplicate-response correction — integrated PR #916, 2026-09-14

PR #916 integra la corrección del residuo `creature_query_cache`, que era una diferencia de
comportamiento, no una autoridad válida de Session. TrinityCore conserva el caché de
bytes serializados en `CreatureTemplate::QueryData` y `WorldSession::HandleCreatureQuery`
responde cada `CMSG_QUERY_CREATURE`, usando el caché solo para construir el payload
(`Handlers/QueryHandler.cpp:71-99`, `World.cpp:1706-1707`). Rust mantenía un
`HashSet<u32>` por Session y silenciaba consultas repetidas; se retiró ese campo y el
handler vuelve a responder cada petición. La regresión focal exige dos respuestas para
dos consultas iguales. El snapshot sintáctico se regeneró con el checker oficial y la
clasificación existente de `represented_player_unit_values_updates_delivered_like_cpp`
se registra bajo la familia de publicación Map/visibility. El ledger conserva 647 campos
(220 producción, 427 fixtures) y reduce el residual exacto a 5; la integración pendiente
debe mantener explícita la diferencia de `CacheDataQueries` si se implementa más adelante.

## P4 Map publication delivery guards — integrated PR #918, 2026-09-14

The fresh audit classifies `represented_capture_point_removed_delivered_like_cpp` and
`represented_dynamic_object_values_updates_delivered_like_cpp` as valid Session
publication fences in `map_runtime_creature_gameobject_and_visibility`. C++ emits the
first from `GameObject::Delete` (`Entities/GameObject/GameObject.cpp:1746-1756`) and
constructs the second through `WorldObjectChangeAccumulator`
(`Entities/Object/Object.cpp:3654-3717`) before `Map::SendObjectUpdates`
publishes updates (`Maps/Map.cpp:1929-1948`). Rust consumes only canonical map
summaries in `session/movement/movement_publication.rs:175-208` and
`session/instances/map_key.rs:453-628`; generation/GUID/fingerprint dedupe is
receiver-local and does not duplicate entity authority. The integrated PR #918 leaves three
production residuals.

Those residuals are not one refactor: instance reset times require moving the
Player `_instanceResetTimes` load/check/add/save contract
(`Player.cpp:1116-1125,19190-19198,27937-28010`); locked encounter loot needs a
canonical InstanceLockMgr query plus injected DungeonEncounter catalog for
`Player::IsLockedToDungeonEncounter` (`Player.cpp:20725-20748`); and seer requires
moving the Player `m_seer`/SetViewpoint lifecycle and every visibility consumer
(`Player.h:2417,2423`, `Player.cpp:298-300,25344-25395`).

## P2 Player dungeon-encounter lock query — integrated PR #921, 2026-09-14

PR #921 completes the encounter-lock authority cut. The immutable
`DungeonEncounterStore` is installed through the existing
`SessionWorldCatalogCapabilitiesLikeCpp` bundle; the Session binding resolves the
player's unique canonical map/difficulty and reads the shared `InstanceLockMgr`
completed-encounter mask, matching `Player::IsLockedToDungeonEncounter`
(`Player.cpp:20725-20748`). Unknown rows and absent active locks are unlocked;
missing or ambiguous authority is indeterminate and production loot fails closed.
`represented_locked_dungeon_encounters` is `cfg(test)` fixture input only, and both
creature and GameObject encounter-loot consumers use the typed query. Focused lock,
loot and composition tests, cargo checks, syntax ownership and architecture checks
pass at merge `d25cbc9161f8affb8c5201a1ad6a653870938969`. The sole exact production
residual under #584 is `Player::m_seer` visibility.

## P2 Player::m_seer canonical visibility projection — integrated PR #923, 2026-09-14

TrinityCore inicializa `Player::m_seer` al propio Player (`Player.cpp:298-300`,
`Player.h:2417-2425`) y sólo lo cambia mediante `SetViewpoint`
(`Player.cpp:25338-25395`); el mapa y la visibilidad consultan ese estado
(`Map.cpp:716-718`, `GridNotifiers.cpp:95-222`). Rust deriva ahora el GUID del
seer de producción desde el `Player` canónico propietario del mapa y su
`ActivePlayerData::FarsightObject`; un valor vacío representa al propio Player.
Los consumidores de visibilidad diferida, movimiento, aggro y consultas de
GameObject/DynamicObject usan esa proyección. El antiguo campo de Session queda
limitado a fixtures `cfg(test)`. `last_observed_farsight_object_like_cpp` es sólo
un cerrojo local del receptor para emitir una vez el paquete de limpieza
FAR_SIGHT tras retirar el viewpoint, no una autoridad de gameplay. Pasan las
regresiones focales FAR_SIGHT (14), GameObject despawn (22), DynamicObject VALUES
(15), `cargo check`, ownership syntax, architecture check y 20 self-tests en el
merge `5f6b1ad8`. Capturas completas, durabilidad DB/relogin y QA viva siguen
siendo gates de gameplay/runtime bajo #41/#63/#584.

## P3.11 canonical Pet visibility CREATE — integrated PR #925, 2026-09-14

The canonical map already indexes `AccessorObjectKind::Pet` in the same Creature
cell family used by TrinityCore's generic `Map::AddToMap` unit path. PR #925
(`a1f66c33`) changes the Session visibility query to use
`with_creature_or_pet_like_cpp`, so an in-world Pet now reaches the existing
`WorldCreature::create_data_from_canonical_like_cpp` and Creature CREATE block after
the normal map, phase, range and detection gates. The C++ anchors are
`Map.cpp:530-610` (`UpdateObjectVisibilityOnCreate`) and `Pet.cpp:69-88`
(`Pet::AddToWorld`/`Unit::AddToWorld`). The regression
`visible_creatures_skip_not_in_world_canonical_objects_like_cpp`
passes together with the affected package check, formatting and diff checks.
This is a visibility projection closure only: Pet AI/movement, summon ownership and
persistence, transport/corpse lifecycle, captures and live QA remain separate
#584/#63 gates; directed Pet DESTROY is covered by the integrated P3.12 slice below.

## P3.12 directed Pet DESTROY — integrated PR #927, 2026-09-14, merge `b7ac63b7`

TrinityCore's generic `Map::RemoveFromMap` calls `WorldObject::DestroyForNearbyPlayers`
while a unit is still attached (`Map.cpp:934-951`, `Object.cpp:3617-3648`);
`Pet::RemoveFromWorld` reaches that Unit path (`Pet.cpp:94-101`). The bounded Rust
change extends the existing pre-erasure recipient capture and deferred directed
DESTROY command from ordinary Creature to Pet, preserving the charmer exclusion,
map-incarnation fence and Session `HaveAtClient` gate. No Pet AI, summon, persistence,
vehicle or transport authority moves.

The focused `wow-map visibility` suite (47 tests) and `wow-world deferred_visibility`
suite (12 tests, including Creature and Pet directed removal) pass with one Cargo job;
format, diff and architecture ratchets remain required before merge.

## P3.13 directed Corpse DESTROY — integrated PR #929, 2026-09-14, merge `028185d8`

The C++ corpse path is the same world-object lifetime: `Corpse::RemoveFromWorld`
delegates to `WorldObject::RemoveFromWorld` (`Corpse.cpp:56`), which calls
`UpdateObjectVisibilityOnDestroy` (`Object.cpp:1023-1029`) while
`Map::RemoveFromMap` still owns the source (`Map.cpp:934-951`). Rust generalizes
the existing directed destroy rail to Corpse, capturing nearby recipients before
erasure and publishing after the map guard with map-incarnation and `HaveAtClient`
fences. Reclaim, persistence, loot, transport/vehicle lifecycle and live QA remain
separate #584/#63 gates.

The 47-test map visibility suite, 12-test deferred-visibility suite, 16-test
mailbox suite, four-package check, formatting/diff checks and architecture ratchets
pass at the merged SHA.

## P4 loaded-grid creature test navigability — integrated PR #931, 2026-09-14, merge `b2545d50`

La auditoría fresca posterior a #929 seleccionó `creature_loaded_grid.rs` para una
división física acotada. La fachada conserva 797 líneas de producción; la fachada
de fixtures y los módulos `builder.rs`/`resolver.rs` conservan las 28 regresiones
con `cfg(test)` explícito. La suite pasa 28/28 y también pasan fmt/diff y los
ratchets de arquitectura. No cambió comportamiento, autoridad ni dependencias; el
terminal físico global y los gates C0–C4/runtime/capturas de #584 siguen abiertos.

## P4 game-event runtime navigability — integrated PR #933, 2026-09-15, merge `ef30bb3e`

La auditoría posterior a #931 dividió el runtime de eventos de 2.643 líneas en
`game_events/mod.rs` y siete módulos de responsabilidad: `unspawn`, `grid`, `spawn`,
`bootstrap`, `scheduler`, `live` y `consume`. Se conservan las reexportaciones y
las 181 regresiones; pasan `cargo check`, fmt/diff y los ratchets de arquitectura.
No cambió comportamiento, autoridad ni dependencias; el terminal físico global y
los gates C0–C4/runtime/capturas de #584 siguen abiertos.

## P2 Player instance-reset owner — integrated PR #919, 2026-09-14

PR #919 moves C++ `_instanceResetTimes` into the canonical
`PlayerGameplayState` (`crates/wow-entities/src/player_gameplay_state.rs:60`) with
named operations in `player/recent_instances.rs`. Map admission creates the canonical
Player before farm-limit checks and entry accounting; login hydration and save projection
read the same owner. The old Session map is `cfg(test)` fixture state only and transfers
once when a fixture owner is materialized. Owner, instance-count, teleport, package and
syntax ownership checks pass. At the #919 checkpoint the exact production residual was the locked-encounter
contract and the Player `m_seer` visibility seam; PR #921 closes the former, leaving
`m_seer` as the only exact production residual.

PR #891 añade el cierre P2 acotado del owner de taxi del Player: el avance de ruta
tras teletransporte y la limpieza del vuelo pasan a ser transiciones nominales sobre
`PlayerTaxi`, siguiendo `PlayerTaxi::NextTaxiDestination` (`PlayerTaxi.h:74`) y
`Player::CleanupAfterTaxiFlight` (`Player.cpp:22019`). Session conserva la admisión
de mapa, la finalización de la spline y los efectos de paquete/aplicación; el
mutador de taxi sin owner queda limitado a fixtures `cfg(test)`. La regresión del
owner, 14 pruebas de taxi, 11 escenarios de acceso canónico, 13 de movimiento, los
checks de paquetes y el ratchet de arquitectura pasan. La generación de rutas,
admisión de nodos, orden de teletransporte, persistencia, capturas y QA viva siguen
siendo gates de gameplay de #584.

PR #893 añade el cierre P2 acotado del owner world-local del Player: las escrituras
de zona/área, autoridad de terreno, hostilidad/timer PvP y outdoors usan
transiciones nominales sobre `PlayerWorldLocalState`, siguiendo
`Player::UpdateZone`, `Player::UpdateArea`, `Player::UpdatePvPState`,
`Player::UpdateContestedPvP` y `WorldObject::IsOutdoors`. Session conserva la
resolución de terreno/catálogo y efectos de rest/aura/paquete/aplicación; el
mutador genérico queda limitado a fixtures `cfg(test)`. Pasan el test del owner,
11 pruebas world-local, chat/zone, world-state, outdoors/spell-state, checks de
paquetes, fmt/diff y ratchet/self-test de arquitectura. La admisión completa,
efectos de aura/quest/rest, persistencia, capturas y QA viva siguen siendo gates
de gameplay de #584.

PR #897 añade el cierre P2 acotado de la presentación de montura del Player: una
transición nominal aplica conjuntamente `MountDisplayID` y `UNIT_FLAG_MOUNT`, siguiendo
`Unit::Mount` / `Unit::Dismount` (`Entities/Unit/Unit.cpp:7822-7865`). Session conserva
los efectos de aura, colisión, vehicle-kit y paquete; el adaptador genérico de
presentación de Unit queda limitado a fixtures desconectados de escala bajo `cfg(test)`.
La regresión del owner, los escenarios de mount, los checks de paquetes y el ratchet
de arquitectura pasan. El gameplay completo de montura, persistencia, capturas y QA
viva siguen siendo gates separados bajo #63/#584.

La auditoría vigente de #63 corrige dos pendientes textuales heredados: los ACK de
velocidad ordinaria ya siguen `MovementHandler.cpp:468-546` en
`session/movement/speed.rs`, y el ACK de knockback ya está integrado por PR #866
según `MovementHandler.cpp:548-559`. Quedan únicamente admisión completa de
seat/offset de vehículos y transportes, ramas cuyo mover o consumidor aún no está
representado, capturas exactas de bytes/orden y QA viva cliente-servidor-DB.

PR #895 añade el cierre P2 acotado del owner RestMgr del Player: las escrituras de
flags de descanso, publicación diferida y reloj pasan por transiciones nominales de
`Player` sobre `PlayerRestState`, siguiendo `RestMgr::SetRestFlag` / `RemoveRestFlag`
(`RestMgr.cpp:95-122`), `RestMgr::_restTime` (`RestMgr.h:86`) y
`Player::SetRestState` (`Player.h:2652`). Session conserva la resolución de
catálogo, la publicación de paquetes y el orden de aplicación; el mutador genérico
queda limitado a fixtures desconectados bajo `cfg(test)`. La regresión del owner,
los escenarios de rest, chat, area-trigger y zona, los checks de paquetes y el ratchet
de arquitectura pasan. El progreso de objetivos de misión, la persistencia durable,
las capturas y la QA viva siguen siendo gates de gameplay bajo #41/#584.

El alcance de esta sincronización es documental. No reabre el análisis completo del
port, no inventa nuevas microissues y no convierte una prueba histórica en evidencia
nueva. Cada macro incluye sus consumidores y sus pruebas; la validación final sigue
la cadencia de `AGENTS.md`.

## 1. Estado que gobierna el plan

**Cabeza de código integrada, 2026-09-14: PR #904**, en `3.4.3` como
`4ad36d420a0f56297390262f3667be1b5fc4ae6f`. PR #887 cierra la superficie de
propiedad de guild del Player después de PR #883 y PR #881. PR #873 corrige el fanout P3.10
integrado por #871 (`304f482b101ff0ac8600854bd1a4ebb72cec2b5d`): Player/Unit
queda exclusivamente en el rail de Session filtrado por receptor y la sesión
revalida el `MapKey` después de soltar el guard de Map. PR #866 queda como la entrega previa de knockback ACK; PR #864 queda como la entrega previa de ACK. PR #862 queda como la entrega previa de ACK anterior. PR #853 queda como la entrega previa de admisión. #787 / PR #792 (`d14a9a67`) y
#584 P2 item-bonus, P2 item-object y P3.1–P3.10 están integrados dentro de esta cabeza.
La entrega de ownership de modificadores de objetos está integrada mediante PR #839
(implementación `ecc67603`) y retira la superficie mutante genérica restante. La coordinación World/Map está
implementada y aceptada localmente en `76369bda`; la corrección mantiene el ACK World pendiente hasta finalizar y
retirar la sesión. El contrato y la evidencia están en el
[checkpoint de sesión](session-578-checkpoint.md#787-resumption-finalization-is-inside-the-world-completion-boundary--2026-09-12).
La secuencia del 11 de septiembre que sigue se conserva como contexto fechado;
no ordena volver a ejecutar entregas ya integradas. La retirada del escritor
legado de criaturas y las fases de mapa no representadas siguen en #584.

La base revisada de la entrega anterior fue `3.4.3` en
`db1250767090a5c951dae96ad6c2a2d5b24873ff`; la base vigente es la cabeza de PR #853
indicada arriba. #133 se cerró el 2026-09-09. Las
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

La entrega funcional #486 está integrada por PR #807 en `86a0eb97` (implementación
`3eafa4b8`): la consulta de
identidad de jugador usa la proyección global equivalente a `CharacterCache`, resuelve
la cuenta objetivo y superpone la identidad conectada desde `PlayerRegistry`; la
composición calienta esa proyección antes de aceptar sesiones y las mutaciones de
administración actualizan el mismo owner después de COMMIT. Sus límites son los
declined names y las mutaciones de undelete/barber aún no representadas. La aceptación
de bytes/captura y la QA viva siguen siendo gates de la issue; la integración remota ya
está satisfecha.

Las macros P3.7, P3.8, P3.9 y P3.10 de núcleo bajo #584 quedaron integradas. P3.7 conecta los planes
de relocalización de criaturas ya calculados con la única vía de publicación de sesiones,
reutiliza las fuentes lejanas de `ObjectUpdater` y aplica el radio de activación por
fuente; P3.8 marca los Players cercanos para el mismo rail cuando un objeto entra o sale
del mapa; P3.9 captura y publica el DESTROY dirigido de una Creature ordinaria con
vallas de encarnación y `HaveAtClient` después de liberar el guard de Map. P3.10
publica los snapshots Player/Unit de `Map::SendObjectUpdates` con filtrado por
receptor y corrige el enqueue canónico de Unit antes de capturar sus máscaras; el
envío ocurre fuera del guard de Map. La corrección #873 retira Player/Unit del
rail genérico de mapa para evitar doble entrega y bytes de propietario sin filtrar,
y vuelve a comprobar la clave de mapa antes de publicar. PR #876 añade el conjunto
separado de `Player::m_visibleTransports` al registro y a la entrega de Transport
VALUES, manteniendo fuera de esta macro CREATE/DESTROY y pasajeros. La macro P2
del último escritor genérico de bonos de objeto también quedó
integrada por PR #816: el estado resuelto se aplica mediante una operación nominal
del runtime propiedad del Player y se retiró el cierre `&mut` de Session. La siguiente
macro incorporada por la auditoría es el cierre nominal de Void Storage descrito en
§4.4; no mezcla estadísticas efectivas ni auras
de #61. La proyección F1 de #61 quedó integrada por PR #851: `Player` posee ahora
un snapshot runtime de estadísticas efectivas y `handlers/character/stats.rs` concentra
la derivación de equipo. PR #857 conecta el consumidor de amenaza de hechizo al AP
canónico con el orden C++ de suma, clamp y multiplicador (`Spell.cpp:5558-5575`,
`Unit.cpp:9165-9180`); PR #858 conecta el consumidor cuerpo a cuerpo a los rangos
base/offhand del Player, derivados por la proyección C++-shaped de `wow-data`. PR #869
añade la admisión exacta de offhand: `haveOffhandWeapon`/`GetWeaponForAttack`
(`Unit.cpp:496`, `Player.cpp:9243-9270`) requieren slot de arma, Item canónico no
roto, y `IsInFeralForm` (`Unit.cpp:8807-8812`) bloquea el ataque.
La issue sigue abierta para productores de auras, consumidores de combate restantes,
cálculo completo, ciclo reversible y la aceptación reversible/captura/QA
descrita en `PORT_PLAN.md`. Después siguen los
residuales P2/P3/P4 por consumidores, el producto #583 y
la auditoría #153. Las
excepciones físicas son individuales y se justifican con la política vigente; no se
crea una issue por fichero, helper o import.

#582 queda cerrado tras su entrega de decodificadores sin matchmaking. #524 conserva
su corrección de orden de consultas integrada por PR #803 y la secuencia WDC4/SQL
table-granular integrada por PR #822/#824 (`7bb9a911`), los overlays de
`SkillLineXTraitTree` por PR #826 (`d934451a`), las eliminaciones por hash por PR
#828 (`276e3981`), el índice genérico de `TraitMgr` por PR #830 (`a9623787`), la
proyección combat/class por PR #832 (`1143ed41`) y la validación de entradas por PR
#834 (`4e3ad8f0`): `SkillLineAbility` termina antes
de `SkillRaceClassInfo`, y TraitMgr construye después la proyección de
`TraitTree`/`SkillLineXTraitTree` con consultas official/custom independientes. La
proyección de `SkillLineXTraitTree` y sus consumidores de hidratación de Player están
implementados con validación de enlaces; profesión, Generic y Combat fallan cerrado
sin árbol enlazado o especialización/clase válida, y las entradas persistidas fallan
cerrado si no existen o exceden `MaxRanks`. PR #836 (merge `0fca1020`, implementación
`d9770755`) añade al mismo dueño la proyección inmutable de nodos, grupos, edges, costes, condiciones y loadouts,
y rechaza en la autoridad de sesión una pareja nodo/entrada que pertenezca a otro árbol.
La macro #524 de validación semántica y fallback determinista quedó integrada por PR #842
(`995cd77f`, implementación `add6650a`): las condiciones, costes/monedas, reglas de
padre/rango y entradas concedidas se evalúan desde hechos canónicos del Player antes de
publicar login. PR #846 (`93fa95a9`, implementación `57116f75`) compone ahora las 24
proyecciones SQL base de Trait/`SpecSetMember` con precedencia official→custom,
`RecordRemoved` final acotado al hash WDC4 y fallo antes de publicar un catálogo
malformado. PR #848 (`179fd5d4`, implementación `95274da1`) añade los overlays de
locale (`trait_definition_locale` y `trait_currency_source_locale`) con las
proyecciones SQL exactas, precedencia official→custom y retención en la capacidad
inmutable del Player. Siguen la cobertura cross-store y la aceptación startup/DB/relogin,
además de los consumidores de locale y EffectPoints; el gasto real, persistencia de
mutaciones y starter builds son gates funcionales posteriores. #486 conserva
la implementación integrada por PR #807 y solo sus gates de captura/QA viva y
mutaciones administrativas no representadas. Estas son líneas funcionales separadas:
no se convierten en trabajo oculto de #584 ni se usan para reabrir macros cerradas.

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
| P2 | Fronteras de Player y operaciones completas | #743 y #735 entregados; continúan los residuales por consumidores |
| P3 | Fases, runtime, lifetime, residencia/incarnation y storage selectivo | #787 entregado; P3.1 retiró el escritor Creature canónico descartado, P3.2 publicó `SendObjectUpdates`, P3.3 corrigió el orden de respawn/condiciones antes de los visitantes, P3.4 conectó la selección cercana de `ObjectUpdater` con producción, P3.5 corrigió el radio de activación por fuente, P3.6 añadió el override de cinemática del Player, P3.7 publica el fanout de visibilidad de relocalización de Creature, P3.8 marca receptores cercanos para admisión/remoción de objetos, P3.9 publica DESTROY dirigido de Creature ordinaria con vallas de encarnación/`HaveAtClient`, P3.10 publica VALUES Player/Unit filtrados por receptor fuera del guard de Map, P3.11 incluye Pets canónicas en la búsqueda de CREATE de Creature y P3.12 extiende DESTROY dirigido a Pets con las mismas vallas; #873 retira esos dos tipos del rail genérico para evitar duplicados y exposición de campos de propietario; #876 proyecta la membresía separada de Transport para VALUES; #878 cierra las transiciones nominales del mount VehicleKit en Player; #881 cierra
las transiciones nominales de aura sobre el Unit-owned AuraSubsystem; el escritor legado, AI/combat, scripts, FlyByCamera, el runtime/owner lifecycle restante de Pet/corpse/transport, flags shared-raid y los gates de captura siguen pendientes bajo #584 |
| P4 | Organización física, excepciones y límites semánticos | #584, acompañado por cada operación; la medición de 31 paths permanece histórica |
| P5 | Producto de módulos M0–M4, nativo/Wasm y Rust/Wasm/C | #583, tras los requisitos core de #584; no bloquea gameplay independiente |
| P6 | Auditoría terminal y evidencia integrada | #153, después de #584 y #583; no absorbe implementación |

La tabla es un mapa técnico de las responsabilidades de `PORT_PLAN.md`/#49. #743,
#735 y #787 ya tienen entregas aceptadas en sus alcances; C0–C4 restante permanece
en #584 y M0–M4 en el producto #583.

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
  progreso de objetivos de #41. **Cerrado por #790 (2026-09-12):** al leer la
  superficie completa se comprobó que la carga
  (`SELECT quest, objective, data FROM character_queststatus_objectives`), cada
  actualización de objetivo y la proyección de guardado usan
  `objective_counts[storage_index]`, y que la forma indexada por id de objetivo no
  tenía ningún escritor. Se retira el campo, su lectura y
  `PlayerQuestObjectiveProgress` en lugar de dejarlos esperando escritores que nunca
  iban a llegar; la operación de progreso de objetivos de #41 sigue siendo el
  contrato pendiente.

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
- Proyección histórica: el contrato de aplicación de encantamiento/equipo que antes
  prestaba `with_bonuses_mut_like_cpp` quedó cerrado por la entrega nominal de
  `ecc67603`, descrita en la sección siguiente. Las reglas de catálogo, efectos
  diferidos, auras y paquetes siguen en `wow-world`; el cierre solo mueve las
  transiciones del estado que C++ ejecuta mientras sostiene el Player.

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

#### Entrega local aceptada — #773

Undécima macro P2: las tres preferencias de dificultad eran miembros públicos y
un cierre genérico que entregaba `&mut u32` de las tres.

- Estado e invariantes en el Player: los tres miembros se cierran a
  `wow-entities` y `crates/wow-entities/src/player/difficulty.rs` recibe los
  accesores por tipo que C++ mantiene en el Player:
  `GetDungeonDifficultyID`/`SetDungeonDifficultyID` (`Player.h:1961`/`:1964`),
  `GetRaidDifficultyID`/`SetRaidDifficultyID` (`:1962`/`:1965`) y
  `GetLegacyRaidDifficultyID`/`SetLegacyRaidDifficultyID` (`:1963`/`:1966`). La
  lectura emparejada y la sustitución de carga siguen en `progression.rs`.
- Los cuatro consumidores de `session/instances/difficulty.rs` nombran qué
  preferencia escriben a través de un ayudante privado que aplica el setter
  canónico o el espejo `#[cfg(test)]` sin handle; tres accesos más en
  `session/directory.rs` y en fixtures de grupo/directorio usan los accesores.
- El mapeo del tipo de dificultad de grupo y del mapa y el envío de paquetes se
  quedan en `wow-world`, que es su dueño.
- Techo físico revisado: `player_tests.rs` pasa de 501 a 502 líneas porque cada
  módulo de escenario nuevo de estas macros añade una entrada de índice; los
  cuerpos viven en los módulos, no en el índice.

Aceptación local: cuatro regresiones nuevas de invariantes en
`player_tests/difficulty.rs`, controles de arquitectura y ownership con delta de
inventario revisado (player/mod.rs +57 producción/+68 test; session/mod.rs +16
producción/-8 test). Sin QA viva ni evidencia de DB/reinicio/relogin.

#### Entrega local aceptada — #775

Duodécima macro P2: la hidratación de detalles de configuración de rasgos
entregaba `&mut PlayerTraitConfigState` al otro lado del límite de crate.

- Estado e invariantes en el Player: `trait_config_row_mut_like_cpp` se retira y
  `PlayerSpellRuntimeState` recibe una transición con nombre que instala de una
  vez el detalle cargado de cada configuración, anclada al camino de login que
  llena las configuraciones del Player (`Player::AddTraitConfig`,
  `Player.h:1836`, leídas por `GetTraitConfig`, `:1837`).
- Las comprobaciones que vivían en el sitio de llamada pasan a ser el invariante
  del propietario: ambos conjuntos de filas autoritativos, una configuración
  entrante por fila almacenada, ids únicos y cada cabecera almacenada igual a la
  entrante. Si algo no cuadra se rechaza la hidratación entera, de modo que un
  detalle no puede describir filas que no se cargaron.
- `session/trait_configs.rs` ya solo da forma al contenido del paquete; el orden
  de la proyección CREATE y las formas de paquete siguen en `wow-world`.

Aceptación local: diez regresiones nuevas de invariantes en
`player_tests/trait_config_hydration.rs`, controles de arquitectura y ownership
con delta de inventario revisado. Techo físico de `player_tests.rs` 502 -> 504
por el módulo de escenario nuevo. Sin QA viva ni evidencia de
DB/reinicio/relogin.

#### Entrega local aceptada — #777

Decimotercera macro P2: el estado de cinemática y película tenía cuatro miembros
públicos y un cierre de subestado en `canonical_access/operations.rs`.

- Estado e invariantes en el Player:
  `crates/wow-entities/src/player/cinematic.rs` posee
  `PlayerCinematicStateLikeCpp` con los miembros cerrados y las transiciones que
  C++ hace sobre el `CinematicMgr` que el Player posee: `BeginCinematic`
  (`Entities/Player/CinematicMgr.h:39`), `NextCinematicCamera`
  (`CinematicMgr.cpp:46`) y `EndCinematic` (`:83`), más el par de película
  detrás de `Player::SendMovieStart`.
- El propietario impone que una secuencia nueva reinicia el recorrido de
  cámaras, que terminar una secuencia suelta sus cámaras y la reporta una sola
  vez, y que el avance de cámara rechaza el borde fuera de rango.
- Departura conservada y nombrada en el módulo, no rededucida en cada llamada:
  C++ lee la cámara del índice que abandona y pre-incrementa sin proteger el
  final del array; RustyCore rechaza ese borde.
- Los siete consumidores de `session/{mod,publication,spell_effects,test_support}`
  nombran su transición y los cuatro campos espejo `#[cfg(test)]` se funden en
  uno del tipo del propietario; el inventario de campos pierde tres. Los envíos
  de paquete siguen en la sesión, que posee la conexión.

Aceptación local: diez regresiones nuevas de invariantes en
`player_tests/cinematic.rs`, controles de arquitectura y ownership con delta de
inventario revisado; techo de `player_tests.rs` 504 -> 506. Sin QA viva ni
evidencia de DB/reinicio/relogin.

#### Entrega local aceptada — #779

Decimocuarta macro P2: el estado de descanso tenía trece miembros públicos y una
cincuentena de escrituras de campo repartidas por la sesión.

- Estado e invariantes en el Player: `crates/wow-entities/src/player/rest.rs`
  posee `PlayerRestState` con los miembros cerrados al módulo Player y las
  transiciones que C++ hace sobre el `RestMgr` que el Player posee
  (`Entities/Player/RestMgr.h`): `SetRestBonus` (`:67`), `HasRestFlag` (`:70`),
  `SetRestFlag` (`:71`), `RemoveRestFlag` (`:72`) y `GetInnTriggerID` (`:75`)
  sobre `_restTime` (`:86`) y `_restFlagMask` (`:89`), con
  `Player::SetRestState` (`Player.h:2652`).
- Las transiciones de bandera conservan sus reglas registradas —el reloj arranca
  con la primera bandera y se detiene con la última, el disparador de taberna se
  va con su bandera— y el par de publicación diferida, el registro de logout y
  el reseteo de localización de la carga pasan a operaciones con nombre.
- Registradas en el módulo como propias de RustyCore, no como miembros C++: la
  sincronización diferida de bandera, porque la bandera de descanso la publica la
  sesión que posee la conexión; la contabilidad de logout, que C++ deriva al
  guardar; y `location_initialized`, que distingue máscara vacía de localización
  nunca establecida.

Aceptación local: trece regresiones nuevas de invariantes en
`player_tests/rest.rs`, controles de arquitectura y ownership con delta de
inventario revisado (player/mod.rs +235 producción/+188 test; session/mod.rs -8
producción/+11 test); techo de `player_tests.rs` 506 -> 508. Sin QA viva ni
evidencia de DB/reinicio/relogin.

#### Entrega local aceptada — #781

Decimoquinta macro P2: el estado local de mundo tenía siete miembros públicos y
una treintena de accesos directos.

- Estado e invariantes en el Player:
  `crates/wow-entities/src/player/world_local.rs` posee
  `PlayerWorldLocalState` con los miembros cerrados y las transiciones que C++
  hace según se mueve el Player: la zona y el área de `Player::UpdateZone` y
  `UpdateArea`, el par `pvpInfo.IsHostile`/`pvpInfo.EndTimer` de
  `Player::UpdatePvPState`, el `m_contestedPvPTimer` de
  `Player::UpdateContestedPvP` y el resultado de `WorldObject::IsOutdoors()`.
- El propietario impone la regla que repetían a mano los tres sitios de
  zona/área: una zona o área distinta de la almacenada deja caer la bandera de
  autoridad del terreno, porque el par que avalaba ya no es el actual.
- Registrados como propios de RustyCore: esa bandera de autoridad, que C++ no
  necesita porque consulta el mapa mientras el Player está en mundo, y el valor
  de exterior con tres estados, cuyo `None` significa que el terreno no lo ha
  establecido para la posición actual.

Aceptación local: diez regresiones nuevas de invariantes en
`player_tests/world_local.rs`, controles de arquitectura y ownership con delta
de inventario revisado (player/mod.rs +166 producción/+145 test; session/mod.rs
-4 producción/+3 test); techo de `player_tests.rs` 508 -> 510. Sin QA viva ni
evidencia de DB/reinicio/relogin.

#### Entrega local aceptada — #783

Decimosexta macro P2 y último subestado del cierre genérico: el estado de campo
de batalla tenía nueve miembros públicos.

- Estado e invariantes en el Player:
  `crates/wow-entities/src/player/battleground.rs` posee
  `PlayerBattlegroundState` con los miembros cerrados y las transiciones de lo
  que C++ guarda en `Player::m_bgData` (`Player.h:2821`, `BGData` en `:976`),
  releído por `InBattleground` (`:2335`) y `GetBattlegroundTypeId` (`:2338`),
  más `Player::SetArenaTeamIdInvited` (`:1956`).
- El propietario impone las dos reglas que repetían los llamadores: un tipo cero
  es `BATTLEGROUND_TYPE_NONE` y limpia el campo de batalla, y una ranura de cola
  sostiene una sola cola, así que instalar la misma ranura dos veces la
  reemplaza en lugar de añadir otra fila.
- Registrados como estado representado cuyo propietario vivo sigue siendo el
  sistema de campos de batalla: el estado (`Battleground::GetStatus()`) y el
  mapa del campo de batalla. Se retiran con esa propiedad viva, no aquí.

Con esto, lo que queda en la entrada del residual ya no es una superficie de
estado: los dos ayudantes que prestan el Player entero son la frontera de acceso
canónico por la que ya pasan todas las transiciones con nombre —resuelven el
handle con comprobación de generación y sostienen el cerrojo del mapa durante
una transición—, y su condición de salida es el trabajo P3 de propiedad en
runtime, no otra macro de encapsulación.

Aceptación local: diez regresiones nuevas de invariantes en
`player_tests/battleground.rs`, controles de arquitectura y ownership con delta
de inventario revisado (player/mod.rs +142 producción/+146 test; session/mod.rs
-7 producción/-1 test); techo de `player_tests.rs` 510 -> 512. Sin QA viva ni
evidencia de DB/reinicio/relogin.

### 4.3 Residuales P2 y paso a P3/P4

Después de #743 y #735 se retiran los accesos genéricos operación por operación. El
residual de #722 queda registrado como cierres que entregan una submatriz `&mut` al
cierre llamador; cambiar el nombre del helper no retira la superficie. #737 está
cerrada e integrada; su frontera de inventario queda cubierta por la reconciliación
de persistencia y no se vuelve a contar como residual arquitectónico.

#### Entrega P2 bajo #584 — operación nominal del runtime de modificadores de objeto

La última superficie genérica concreta era `with_bonuses_mut_like_cpp`, que prestaba
`&mut PlayerItemBonusStateLikeCpp` desde `WorldSession` para que el llamador aplicara
reglas de equipo y encantamiento. En TrinityCore esas mutaciones son parte del Player:
`Player::_ApplyItemBonuses` (`Player.cpp:7688-7975`) y el tramo de estado de
`Player::ApplyEnchantment` (`Player.cpp:13058-13389`).

PR #816 (`db1250767090a5c951dae96ad6c2a2d5b24873ff`) mueve ese tramo resuelto a
`PlayerItemModifierRuntimeStateLikeCpp::apply_enchantment_effect_action_like_cpp`
en `crates/wow-entities/src/player/item_modifiers.rs`. La operación acepta una
acción tipada y aplica solo el estado que el owner puede resolver: modificadores de
unidad, ratings, regeneración, penetración, bloque, daño/tiempo de arma y las marcas
de actualización. La consulta de catálogos, la admisión del objeto, los hechizos de
equipo/uso, auras, valores de actualización y paquetes permanecen en
`crates/wow-world/src/session/player_items`. Las variantes diferidas, desconocidas o
que requieren esos servicios se conservan como explícitamente no aplicadas; no se
inventó una dependencia inversa ni un segundo espejo mutable.

Se retiraron los helpers equivalentes de `session_rules/rules_1.rs` y los consumidores
de producción y fixtures migraron a la operación nominal. El test de owner comprueba escritura y
snapshot independiente; la composición de `wow-world` mantiene los caminos de
modificadores, daño y almacenamiento. La aceptación ejecutada en la rama candidata
incluye 14 tests de `wow-entities`, 16 de entidades de mundo, 129 de Player-items,
`cargo check --locked --tests -p wow-world`, formato/diff y
`VALIDATION_V2_CARGO_JOBS=1 ./tools/validation-v2 quick --base origin/3.4.3` en
8.53 s con un job (`target/validation-v2/manifests/20260913T100604.990747Z-3562620-quick.json`).
El ratchet físico pasa; el ratchet lógico conserva únicamente el drift histórico de
`session/mod.rs`, `wow-map/src/map/mod.rs`, `handlers/character/mod.rs` y
`world-server/src/lib.rs`, sin regenerar sus baselines. El ledger mueve el techo
lógico agregado de `player/mod.rs` a 15.468 líneas de producción y 13.122 de tests,
con la deuda C4 de separación física todavía abierta.

Este corte no afirma paridad completa de estadísticas/auras, paquetes CREATE/DESTROY,
cliente/captura, DB/reinicio/relogin ni cierra #584. El siguiente trabajo debe
seleccionarse por consumidores restantes: catálogos y efectos de item que aún carecen
de comportamiento, otras superficies P2 medidas, o una entrega P3 con contrato de
runtime; no se crea otra macro solo para renombrar este owner.

#### Entrega P2 bajo #584 — cierre del runtime de objetos de inventario

La auditoría corregida de #737 dejó un único residual de acceso genérico en producción:
`item_objects_mut()` prestaba un `&mut Item` a un cierre llamador. Los insert/remove
concretos ya habían sido retirados por #740; el residual afectaba a los consumidores de
inventario, vendor, loot, recompensas, durabilidad, hechizos y void storage. Los anclajes
de propiedad de TrinityCore son `Player::_StoreItem` (`Player.cpp:11246`),
`Player::VisualizeItem` (`:11510`), `Player::RemoveItem` (`:11553`) y
`Player::QuickEquipItem` (`:11476`): el cambio de slot lleva consigo los campos del
objeto, su estado y la publicación posterior.

La entrega candidata `23a7fe16` concentra esa mutación en
`wow_entities::PlayerInventoryRuntime`. `ItemObjectUpdateLikeCpp` es un conjunto cerrado
de comandos para contenedor, slot, cantidad, durabilidad, flags, binding, gemas,
encantamientos, estado, reemplazo y relocalización de bolsa; el owner resuelve el
`HashMap` y nunca expone `&mut Item`. La apertura de regalo envuelto es una operación
nominal que conserva la durabilidad previa para su proyección de persistencia. Todos los
consumidores de producción usan estas operaciones; el helper de cierre y los accesos
directos al mapa quedan bajo `cfg(test)` para fixtures existentes.

La prueba del owner verifica aplicación y rechazo de GUID inexistente; la prueba de
regalo verifica que la durabilidad se conserva para persistencia. El catálogo de ownership
se sincroniza con la nueva operación, el campo `trait_tree_skill_line_index`, el comando
`DestroyVisibleCreatureLikeCpp` ya integrado y las métricas actuales. No se añade campo,
mirror, lock, task, reloj ni dependencia entre crates. La aceptación local de esta entrega
incluye `cargo check --locked --tests -p wow-entities -p wow-world`, los tests focalizados
de `wow-entities` (24) y `wow-world` (5), formato, diff y los checks de arquitectura y
ownership con un solo job de Cargo; la validación-v2 final y la publicación quedaron
registradas para la entrega integrada.

Este cierre retira la superficie arquitectónica genérica de #737, pero no declara paridad
completa de item use/effects, estadísticas, auras, bytes de cliente ni durabilidad real
con DB/reinicio/relogin. Las funciones de gameplay o datos que aún falten se asignan a su
macro funcional; no se reabre #737 ni se crea una issue por cada variante del comando.

#### Entrega integrada — runtime de modificadores del Player

La auditoría de `origin/3.4.3` en `ef82beeb` encontró un residual P2 distinto del
item-object ya integrado: `WorldSession::mutate_player_item_modifier_runtime_like_cpp`
(`session/player_items/modifiers.rs`) todavía prestaba un cierre `FnOnce(&mut
PlayerItemModifierRuntimeStateLikeCpp)` a ocho consumidores de item-set, encantamiento y
valoración. El estado ya tiene como propietario semántico al `Player`; la superficie
genérica era el último acceso que permitía que el llamador mutara el contenedor completo.

La entrega `ecc67603`, integrada por PR #839 como `cc0559980a4232ab5743affaaa2babfedffdfcf3`,
añade operaciones nominales en `wow-entities::Player` para snapshot,
añadir/quitar piezas y bonus de conjuntos, eliminar efectos vacíos, instalar caps de nivel,
reiniciar bonuses y aplicar una acción de encantamiento. Los consumidores de
`session/player_items/{items,modifiers,valuation}.rs` y sus fixtures llaman esas
operaciones a través del acceso canónico comprobado; el fallback de sesión queda limitado
a `cfg(test)` y no expone el contenedor a un cierre de producción. El orden de catálogo,
admisión, auras, paquetes y publicación no cambia: permanecen en `wow-world`, por lo que
esta entrega no intenta resolver la funcionalidad de estadísticas efectivas de #61.

Las anclas de comportamiento son `AddItemsSetItem`/`RemoveItemsSetItem`
(`Entities/Item/Item.cpp:57,146,192`), `_ApplyItemBonuses`
(`Player.cpp:7688-7975`) y el tramo de estado de `ApplyEnchantment`
(`Player.cpp:13058-13389`). Se añadió una regresión de owner directo en
`player_tests/item_modifiers.rs` y se conservaron las regresiones de detached/stale
Player y de daño que ejercitan los adaptadores de sesión. `cargo check -p wow-world`
con un job y los dos tests focalizados de `wow-world` pasan; el test de owner de
`wow-entities` se ejecuta junto con la suite del crate. El baseline de
`session-ownership-check` se actualiza solo por las operaciones nuevas y la retirada del
helper genérico, con el delta revisado. La validación final `validation-v2 final` pasó
el 2026-09-13 sobre el candidato que originó PR #839; su manifiesto es
`target/validation-v2/manifests/20260913T165720.613650Z-3851477-final.json` y la suite
de `wow-entities`/`wow-world` terminó con 3875 tests correctos, uno ignorado y cero
fallos. La evidencia no amplía el alcance funcional descrito abajo.

Este macro no incluye TraitMgr funcional (#524), escritor Creature legado, AI/combat,
auras, estadísticas, DB/reinicio/relogin ni módulos #583. #524 queda como la siguiente
macro funcional amplia después de este cierre: su catálogo base y sus overlays de locale
están integrados por PR #846 y PR #848; requiere autoridad de monedas y condiciones,
gasto persistente, consumidores de EffectPoints/locale, cobertura cross-store y
aceptación de startup/DB/relogin. El escritor Creature se mantiene retenido hasta poder migrar todos
sus consumidores y su fanout sin crear un segundo writer.

#### Entrega P2 bajo #584 — ownership de Void Storage

La auditoría posterior a la entrega de item-object encontró que el último cierre mutable
genérico de esta familia prestaba el vector de slots y el marcador de carga desde
`WorldSession`. PR #844 (`6f42782f`, implementación `77e2c4b2`) mueve al `Player` la
superficie completa de `Player::_voidStorageItems`: normalización a 160 slots,
clear/load/mark, free-slot, lookup por id/slot, add, delete y swap. Las anclas de
TrinityCore son `Player.cpp:18334`, `20002` y `28025-28098`.

La sesión conserva la admisión de plantillas, la consulta y orden de persistencia, la
aplicación de apariencias y la codificación/publicación de paquetes; el fallback
desconectado queda limitado a fixtures `cfg(test)`. Las regresiones de invariantes del
owner, los 29 tests Void Storage de `wow-world`, `cargo check --tests` y el guardrail de
arquitectura pasan con un job. No se afirma durabilidad real de DB/reinicio/relogin ni se
cierra #584. El siguiente trabajo funcional amplio de #524 queda acotado a la aceptación
cross-store y startup/DB/restart/relogin de la composición ya integrada por PR #846 y
PR #848, y después a los consumidores funcionales de locale/EffectPoints, gasto,
mutación y starter builds.

#### Entrega P2 bajo #584 — ownership del mount VehicleKit del Player

La auditoría posterior a Transport VALUES encontró el último acceso de producción
genérico al kit de vehículo montado por un Player. PR #878 (`d8cb0594`, implementación
`ceb58c9a`) mueve las transiciones nominales a
`crates/wow-entities/src/player/vehicle.rs`, siguiendo
`Unit::CreateVehicleKit`, `Unit::RemoveVehicleKit` y `Unit::GetVehicleKit`
(`Unit.cpp:11304-11323`). `Player` posee ahora install, snapshot, clear,
uninstall/removal y expulsión de pasajero desde un asiento ejectable; Session
conserva admisión de plantilla, aura, paquetes y publicación/presentación.

La regresión del ciclo del owner y las tres regresiones de expulsión pasan junto a
los checks de `wow-entities`/`wow-world`, formato/diff y el ratchet de arquitectura.
El fallback sin Player queda limitado a fixtures `cfg(test)`. Esta entrega no mezcla
la corrección funcional completa de vehículos: admisión de seat/offset, ciclo de
pasajeros, CREATE/DESTROY, Pet/corpse/Transport, persistencia, capturas ni QA viva
siguen siendo límites separados de #584/#63. El siguiente macro de #584 vuelve a
seleccionarse solo después de una auditoría C0–C4 nueva y completa.

#### Entrega P2 bajo #584 — ownership de TradeData del Player

La auditoría de las superficies P2 restantes encontró que el estado representado de
comercio seguía escribiéndose mediante un cierre genérico de `WorldSession`, aunque
TrinityCore lo posee en `Player::m_trade` (`Player.h:2998`). La entrega candidata
añade `crates/wow-entities/src/player/trade.rs` y nombra las transiciones completas
de apertura/cierre, índices de cliente/servidor, aceptación, oro, slots de objetos y
hechizo, con anclas `TradeHandler.cpp:694-695`, `Player.cpp:12864-12879` y
`TradeData.cpp:58-150`.

Session conserva admisión de dinero e inventario, validación de catálogos, paquetes y
el buzón del participante remoto. El cierre de estado completo queda solo para
fixtures sin handle bajo `cfg(test)`, sin segundo owner, lock, reloj o espejo de
producción. Las regresiones del owner cubren las transiciones y el caso de oro no
asequible; la suite social, checks, formato/diff y ratchet de arquitectura pasan.
La liquidación durable, las capturas y la QA viva siguen siendo límites funcionales.

#### Entrega P2 bajo #584 — ownership de guild membership del Player

PR #885 integra en `19dea8e078f5b7bec827532cefb8f46911e332e7` la siguiente auditoría
C0–C4: la membresía de guild y la invitación pendiente se modificaban a través de
un cierre compuesto de `WorldSession`, aunque TrinityCore las conserva en
`Player::SetInGuild` (`Player.cpp:7216`), `SetGuildIdInvited` y `SetGuildRank`
(`Player.h:1939,1943`). La entrega añade operaciones nombradas al módulo social
privado de `Player` para membresía,
invitación, rango, limpieza de invitación y snapshot; retira el instalador compuesto.

Session conserva los efectos GuildMgr/cache, protocolo y aplicación. El mutador de
estado completo queda solo para fixtures sin handle bajo `cfg(test)`, sin segundo owner,
lock, reloj o espejo. Las regresiones del owner, la suite social, la suite de handlers
de guild, checks, formato/diff y ratchet de arquitectura pasan. Persistencia del
manager, capturas y QA viva siguen siendo límites funcionales.

#### Entrega P2 bajo #584 — ownership de Battleground del Player

PR #887 integra en `72f6a3fa87d00f9319c1cfa626f7a10345fc9654` la siguiente
auditoría C0–C4: el tipo/mapa, estado, cola y la invitación de arena representados
se escribían desde un mutador compuesto de `WorldSession`, aunque TrinityCore los
conserva en `Player::m_bgData`/`BGData` (`Player.h:976,2821`), los lee mediante
`InBattleground`/`GetBattlegroundTypeId` (`Player.h:2335-2338`), establece el id
representado en `Player.cpp:24258-24262` y mantiene la invitación en
`SetArenaTeamIdInvited` (`Player.h:1956`).

`player/battleground.rs` nombra las transiciones del owner y retira el instalador
compuesto de producción. Session conserva admisión de cola, matchmaking, lifecycle,
paquetes y efectos de aplicación; el mutador completo queda solo para fixtures sin
handle bajo `cfg(test)`. Las pruebas del owner, el escenario canónico de ownership,
la suite PVP, checks de paquetes, formato/diff y ratchet de arquitectura pasan.
La funcionalidad completa de cola/matchmaking/lifecycle, persistencia, capturas y QA
viva sigue siendo una frontera de gameplay separada.

#### Entrega P2 bajo #584 — ownership de capacidades persistentes del Player

PR #889 integra en `3.4.3` (`e37570c4e1e9feee04aadac6d485f5d1f314ced1`,
implementación `4066e261`) el módulo privado
`wow-entities/src/player/persistent_capabilities.rs`. Los flags de login y las
máscaras de proficiency de arma/armadura se modifican mediante operaciones
nominales del `Player`, siguiendo `Player.h:1433-1434,2474`; el instalador
composite anterior queda retirado. `Session` conserva la proyección de persistencia,
los paquetes y la admisión de aplicación, con el adaptador genérico limitado a
fixtures `cfg(test)`. Pasan los tests del owner, persistencia, spell-state, checks
de paquetes, formato/diff y ownership arquitectónico. No se afirma durabilidad
DB/reinicio/relogin, captura ni QA viva; #584 sigue abierto.

#### Contraste P3 de composición y fases — revisión acotada 2026-09-12

**Corrección de la primera versión de esta sección (misma fecha).** La versión
publicada en `31cb68e7` afirmaba que RustyCore no tiene temporizador compartido de
mapas, ni paso de descarga, ni barrera previa a la fase retrasada, y que el orden de
fases de `Map::Update` no está representado. Al abrir el macro correspondiente se
comprobó lo contrario en el código: `wow_map::MapManager::update_with_optional_pool_update_context`
(`crates/wow-map/src/manager/state_2.rs:426-516`) avanza `timer`, sale si no ha
pasado, por cada mapa comprueba `can_unload` y lo destruye antes de actualizar,
programa en el `MapUpdater` cuando está activo, espera con `wait()`, elimina los
destruidos y recorre otra vez con `delayed_update`; y el bucle canónico entra por ahí
(`canonical_map_update_tick_set_inactive_like_cpp`,
`crates/world-server/src/runtime/map.rs:1173`). `Map::update`
(`crates/wow-map/src/manager/state_1.rs:471`) también documenta el orden C++ y
representa parte de sus fases con sus huecos anotados. Aquello fue una lectura
insuficiente convertida en afirmación; esta versión la sustituye.

Lectura de fuentes: C++ `Map::Update` (`Maps/Map.cpp:666-813`), `MapManager::Update`
(`Maps/MapManager.cpp:287-318`) y los filtros de sesión de
`Server/WorldSession.cpp:64-108`; RustyCore en `f2fb8955`. Es una revisión de fuentes
con fecha: no ejecuta el runtime ni prueba alcanzabilidad, orden real ni seguridad de
cerrojos.

Lo que sí está representado hoy, verificado en el código citado arriba: el reloj
compartido de `MapManager`, la decisión de descarga antes de actualizar, la barrera
del actualizador y la segunda pasada de `delayed_update`, además del esqueleto de
fases de `Map::update` con sus huecos declarados.

Las diferencias que quedan, y que son el trabajo P3 real:

1. **Un segundo escritor de producción fuera de la actualización de mapa.** El bucle
   `legacy_creature_runtime` avanza ciclo de vida, movimiento, aggro, hechizo y melé
   de criatura en su propio intervalo, mientras C++ visita esas criaturas dentro de
   `Map::Update` a través de `ObjectUpdater`. Es el paso 7 de
   `adr-runtime-tick-ownership.md` —migrar la fuente de verdad hacia
   `wow_map::MapManager` y retirar el legado— y no una carencia de composición del
   `MapManager`.
2. **La sesión se actualiza fuera del mapa.** `Map::Update` recorre los jugadores del
   mapa y llama a `session->Update(diff, MapSessionFilter)`; el filtro
   (`WorldSession.cpp:64`) admite `PROCESS_INPLACE`, rechaza `PROCESS_THREADUNSAFE` y
   exige `IsInWorld`, dejando el resto para `World::UpdateSessions`
   (`WorldSessionFilter`, `:85`). En RustyCore cada sesión corre en su propia tarea
   (`session_factory.rs`) con su diff y su espera de 50 ms cuando no hubo paquetes.
   **Corrección (2026-09-13):** el contrato ya tiene consumidor de producción. El
   driver coordinado selecciona la cabecera FIFO con `MapSessionFilter` o
   `WorldSessionFilter` mediante `run_phase_packet_pass_like_cpp`; la sesión no
   procesa dos veces una cabecera y deja la ineligible para la otra fase. #787 lo
   integró con permisos, incarnación, reemplazo y apagado; la descripción anterior
   que decía que el driver drenaba toda la cola era obsoleta.
3. **Las fases de `Map::Update` que siguen incompletas** están anotadas en el código.
   P3.4 ya conecta la visita cercana y el plan de objetos con el ciclo de producción:
   `ObjectUpdater` recibe solo GUIDs in-world seleccionados desde Players, viewpoints,
   referencias lejanas y objetos activos. Transportes conservan su bucle separado;
   P3.5 ya aplica la distancia de activación específica de Creature/Pet para fuentes
   inactivas mediante `m_SightDistance`, y P3.6 aplica el override de cinemática del
   Player mediante `max(DEFAULT_VISIBILITY_INSTANCE, Map::GetVisibilityRange())` cuando
   el cursor de cámara representado está activo. **P3.7** consume
   `CreatureRelocationVisibilityPlan.player_visibility_updates` en
   `retain_selected_player_visibility_refreshes_like_cpp`, coalesce un único intent por
   Player y lo entrega por la vía existente de residencia/incarnation y sesión diferida.
   P3.8 extiende esa misma vía a los receptores cercanos de `Map::AddToMap` y
   `Map::RemoveFromMap`, marcando `NOTIFY_VISIBILITY_CHANGED` antes de insertar/retirar
   el objeto y dejando la entrega fuera del guard de Map. P3.9 añade la publicación
   dirigida de DESTROY para Creature ordinaria con vallas de encarnación y
   `HaveAtClient`. P3.10 añade VALUES Player/Unit filtrados por receptor después de
   `Map::SendObjectUpdates`, también fuera del guard; la corrección #873 evita que
   Player/Unit entren además en el rail genérico y revalida la clave de mapa antes
   del envío. CREATE, Pet/corpse/transport,
   FlyByCamera, scripts y AI/combat
   siguen teniendo contratos propios y no se introducen en esta macro.

#### Selección P3.1 bajo #584 — retirada del escritor sombra de Creature

La auditoría de fuentes y consumidores del 2026-09-12 cierra la primera selección
implementable. C++ tiene un único `Trinity::ObjectUpdater` dentro de
`Map::Update` (`Maps/Map.cpp:695-754`; `Grids/Notifiers.cpp:258-264`), que llama a
`Creature::Update` para los objetos admitidos por las celdas activas. En RustyCore
la visita canónica (`wow-map/src/map/update.rs:361-490`) ejecuta
`Creature::runtime_update_plan`, pero `ManagedMap::update_after_sessions_like_cpp`
(`wow-map/src/manager/state_1.rs:506-540`) descarta el plan y no publica sus acciones.
El comportamiento efectivo continúa en `run_legacy_creature_runtime_tick_and_deliver_once_like_cpp`
(`world-server/src/runtime/delivery.rs:1490-1700`), que resuelve ciclo de vida,
movimiento, aggro, hechizos y melé una vez y entrega después de soltar los cerrojos.
Los puentes de carga/condiciones (`runtime/game_events.rs:382-470, 1100-1215`)
siguen siendo sincronización de representación, no un segundo reloj.

La escritura canónica descartada avanza temporizadores y registra acciones que nadie
consume; por eso no es evidencia de paridad y sí una escritura sombra que puede
desalinear la representación canónica del dueño efectivo. La primera macro de #584
queda fijada así:

1. Hacer explícito, por tick, el dueño de la fase Creature (`CanonicalMap` o
   `ExternalRuntime`) en `wow-map` y conservar el modo canónico para las pruebas y
   para la futura migración.
2. Cuando el servidor mantiene el escritor legacy/session, seleccionar
   `ExternalRuntime` en la reanudación canónica. La fase Creature canónica se omite
   sin mutar temporizadores ni perder planes; las demás fases de `Map::Update` y los
   puentes de spawn/respawn permanecen sin cambios.
3. Registrar el dueño seleccionado y cubrir la decisión con una regresión de
   `MapManager` que demuestre que un dueño externo no produce un plan descartado,
   mientras el test canónico existente conserva la visita explícita cuando el modo
   es `CanonicalMap`.

El contrato incluye `MapManager`/`MapUpdater`, `ManagedMap`, el productor canónico y
sus tests. No retira todavía el escritor legacy, no migra IA/combat, no cambia el
orden de paquetes ni declara representadas las visitas por celda. La siguiente
macro solo se abre después de que esta retirada esté integrada y se audite el
consumidor que permita trasladar una transición completa al mapa canónico.

#### Entrega P3.7 bajo #584 — fanout de visibilidad de Creature

La auditoría comparó `Map.cpp:666-767,797-805,830-905`,
`GridNotifiers.cpp:137-234` y `WorldObject::GetGridActivationRange`
(`Object.cpp:1433-1450`) con el camino Rust actual. El plan de relocalización ya
calculaba `CreatureRelocationVisibilityPlan.player_visibility_updates`, pero el
owner de Player solo consumía `player_plans`; una Creature reubicada podía por tanto
no provocar ninguna actualización para los Players que la observan.

La entrega `a130d9da` hace tres cambios acotados:

1. `Map::map_update_player_sources_for_current_tick_like_cpp` es la única construcción
   de fuentes de Player para `ObjectUpdater` y `ProcessRelocationNotifies`. Incluye
   viewpoint, combate PvE lejano, casters de aura, summons y referencias activas; la
   visita usa `grid_activation_range_for_guid_like_cpp` para cada centro.
2. `MapManager::retain_selected_player_visibility_refreshes_like_cpp` transforma los
   Players afectados por cada plan de Creature en intents de publicación, junto con los
   planes de Player ya existentes, y ordena/deduplica por GUID antes de crear la única
   obligación coalescida del owner.
3. La entrega sigue fuera del cerrojo de Map por
   `world-server/src/runtime/map/update_loop.rs:338-342`,
   `runtime/deferred_visibility.rs` y `WorldSession::apply_deferred_player_visibility...`;
   se conservan residence revision, incarnation, viewpoint, backpressure y descarte
   de sesiones obsoletas. No se ejecutan AI relocation checks ni se inventa un ledger
   de GUIDs de cliente en Map.

La aceptación local cubre dos criaturas que afectan al mismo Player y producen un solo
intent (`manager::player_owner::visibility::creature_relocation_exports_one_coalesced_refresh_for_each_affected_player`),
la reutilización de fuentes lejanas y radio por fuente
(`map::tests::visibility::relocation_reuses_object_updater_far_player_sources_like_cpp`),
las regresiones de planes/notifiers existentes y los caminos de incarnation, detach,
queue y desconexión del owner de visibilidad. El cambio no afirma todavía la entrega
de paquetes CREATE/DESTROY por criatura individual ni la paridad live de cliente,
captura, DB/reinicio/relogin o AI/combat.

#### Entrega P3.8 bajo #584 — intents de visibilidad del ciclo de vida de objetos

La auditoría posterior a P3.7 encontró que la selección de receptores también faltaba
en las entradas y salidas de objetos del mapa. TrinityCore ejecuta el recorrido de
visibilidad del objeto en `Map::AddToMap` y `Map::RemoveFromMap`
(`Maps/Map.cpp:530-610,933-951`) mediante las operaciones por defecto de
`WorldObject` (`Entities/Object/Object.h:703-704`). RustyCore ya podía recalcular la
visibilidad exacta desde la sesión, pero ningún camino de admisión/remoción marcaba a
los Players cercanos para que ese rail se activara.

P3.8 añade `Map::mark_nearby_players_for_visibility_like_cpp` en
`wow-map/src/map/visibility.rs`. Cada variante de `add_map_object_record_to_map_like_cpp`
la llama después de que el objeto está en-world, y `remove_from_map_like_cpp_inner` la
llama antes de borrar un origen en-world. El helper toma una instantánea de posición y
alcance, visita las celdas cercanas, ordena/deduplica Players canónicos y solo marca
`ObjectNotifyFlags::VISIBILITY_CHANGED`; no presta referencias, no crea un segundo
estado y no entrega paquetes bajo la mutación. `MapManager` y el Session existente
siguen coalesciendo y validando residencia/incarnation/viewpoint fuera del cerrojo.

La aceptación cubre `add_to_map_marks_nearby_players_for_deferred_visibility_like_cpp`
y `remove_from_map_marks_nearby_players_for_deferred_visibility_like_cpp`, con Player
canónico cercano, fuente Creature y comprobación de retirada. El alcance es el hueco de
selección de receptores: la serialización exacta por objeto, transportes, capturas y QA
live/DB/reinicio/relogin siguen siendo gates posteriores. Los campos de resultado que
describen la llamada síncrona C++ continúan marcando ese paso directo como runtime gap;
la implementación Rust entrega su equivalente en la fase diferida para respetar la
frontera de locks.

#### Entrega P3.9 bajo #584 — DESTROY dirigido de Creature ordinaria

La auditoría de P3.8 dejó separado el paso que TrinityCore ejecuta en
`WorldObject::DestroyForNearbyPlayers` (`Entities/Object/Object.cpp:3617-3655`):
retirar de forma dirigida solo la Creature ordinaria que el receptor tenía en
`m_clientGUIDs`, excluyendo al charmer, antes de que el objeto desaparezca del mapa.
PR #820 (`62c1369f`, implementación `8ab62574`) conserva esa responsabilidad en una sola
cadena. `Map::RemoveFromMap` captura los Players cercanos y el charmer mientras la
Creature sigue adjunta, y el map tick añade el `map_incarnation` al resultado sin
serializar ni entregar bajo el guard.

Después de liberar el guard, `world-server` resuelve el registro actual de cada Player
y publica `SessionCommand::DestroyVisibleCreatureLikeCpp` en el rail durable. La
Session comprueba estado, mapa, instancia, encarnación y `HaveAtClient` en
`client_visible_guids_like_cpp`; en el caso válido elimina el GUID y emite el bloque
`SMSG_UPDATE_OBJECT` DESTROY de forma atómica. El drenaje del rail mantiene este
comando por delante de un refresh de visibilidad coalescido, incluso con un paquete
gated anterior en cola. El alcance no incluye CREATE, Pet, corpse/transport, shared
vision, captura ni QA live/DB/reinicio/relogin.

La aceptación son las pruebas de captura/remoción de `wow-map`, las pruebas positiva,
stale e invisible de `wow-world` y la prueba de la valla de encarnación del mailbox;
`cargo check` de `wow-map`/`world-server`, formato y diff pasan con un job. Tras la
integración se debe repetir el perfil `validation-v2 final` y actualizar este plan
con el SHA de integración; el siguiente macro se elige solo después de auditar los
residuales medidos.

#### Entrega P3.10 bajo #584 — VALUES Player/Unit con filtrado por receptor

PR #871 (`304f482b`, implementación `30157c1f`) integra el consumidor de los
snapshots Player/Unit que `Map::SendObjectUpdates` captura antes de limpiar sus
máscaras. Los setters de `Unit` vuelven a encolar el objeto canónico mediante
`AddToObjectUpdateIfNeeded`, y la sesión aplica el filtrado C++ disponible: el
Player propio conserva `ActivePlayerData`/campos de propietario, mientras un
observador recibe solo los campos permitidos; Creature/Pet usa el mismo límite de
mapa, instancia, encarnación, fase, distancia y `HaveAtClient`. La serialización se
prepara antes de enviar y ningún guard de Map cruza la entrega.

La regresión cubre tres escenarios de `wow-world`, 27 pruebas de `wow-entities`,
`cargo check` de `world-server`, formato/diff, arquitectura y `validation-v2 quick`
en `target/validation-v2/manifests/20260914T072651.033982Z-371881-quick.json`.
El snapshot compartido de grupo/raid, las capturas exactas, la cobertura completa
CREATE/Pet/corpse/transport y la QA de DB/restart/relogin siguen fuera del alcance.
El siguiente macro de #584 requiere una auditoría nueva; esta entrega no migra el
escritor Creature legacy ni cierra el coordinador.

La corrección posterior quedó integrada por PR #873 (`bd5b13d4`) desde
`5586997d`. El rail genérico ya no recibe Player/Unit, por lo que no puede duplicar
la publicación ni transportar bytes de propietario a observadores; la sesión falla
cerrada si el `MapKey` admitido cambió mientras se soltó el lock. Pasan las dos
regresiones focalizadas (`map_send_object_updates_` de `wow-world` y la entrega
genérica de `world-server`), `cargo check --locked -p world-server`, formato/diff y
`python3 tools/architecture/check_architecture.py check` con un job.

#### Entrega P3.10b bajo #584 — Transport VALUES con membresía separada

La auditoría posterior encontró que `Map::SendObjectUpdates` ya convertía
`GameObject|Transport` VALUES en el rail genérico, pero ese rail solo consultaba
`Player::m_clientGUIDs`; RustyCore mantiene los Transport en el conjunto separado
que C++ llama `Player::m_visibleTransports`. Por ello PR #876 (`ed92d14f`,
implementación `0c8e0f69`) añade ese conjunto al contrato de registro y al
`PlayerRuntimeRecipient`, sin convertirlo en una segunda autoridad de objetos.

El delivery de `world-server` selecciona la membresía de Transport únicamente para
GUIDs MO transport y conserva la visibilidad ordinaria para cualquier otra familia.
El consumidor de Session repite la valla antes de enviar el raw packet, de modo que
un comando atrasado o una pertenencia limpiada se rechazan. Las regresiones focales
de world-server y wow-world cubren receptor visible/no visible y ausencia/presencia/
limpieza de la pertenencia; `cargo check`, formato/diff y el chequeo de arquitectura
completo pasan. CREATE/DESTROY de Transport, pasajeros, capturas exactas y QA viva
de DB/reinicio/relogin siguen siendo límites explícitos de #584/#63.

#### Entrega F1 bajo #61 — consumidores de AP y rango de arma

PR #857 (`fd302c35`) hace que la amenaza inicial de hechizo consuma el snapshot
efectivo de AP propiedad del Player, conservando la suma de modificadores, el clamp
no negativo y el orden de multiplicadores de C++ (`Spell.cpp:5558-5575`,
`Unit.cpp:9165-9180`). PR #858 (`cda7f8a0`) hace que el tick cuerpo a cuerpo tome
los rangos base/offhand del mismo snapshot antes de la transición mutable del reloj
de Unit; la derivación de item/AP/retardo vive en una proyección pura de `wow-data`.
PR #869 (`abd396a0`) añade la admisión canónica de offhand y el veto de forma feral
en el mismo tick, con pruebas de ausencia, objeto usable, objeto roto y forma
feral. Los consumidores de paquetes y persistencia permanecen separados del snapshot.

Las regresiones enfocadas de `wow-world`, `wow-entities` y la validación final de
cada PR pasan en sus manifiestos registrados. Estos son cortes acotados: no cierran
#61 ni prueban productores de aura, matemática completa de daño/regen/expertise/
penetration, admisión de todas las armas, ciclo reversible de equipo o captura/QA
viva.

#### Entrega F1 bajo #63 — admisión de teleport y MoveSpline

La auditoría de `MovementHandler::HandleMovementOpcode` confirmó dos retornos
tempranos de TrinityCore que deben preceder a toda mutación: un `Player` que está
siendo teletransportado (`Player.h:2170-2172`, combinación de teleport cercano y
lejano) y cualquier mover cuyo `movespline` no esté finalizado
(`MovementHandler.cpp:305-335`). PR #853 (`7c3add2f`, implementación `3af90ec2`)
los aplica en `handlers/movement/ops_1.rs` antes de limpiar emote, mover posición o
publicar estado. `session/movement/state.rs` consulta primero el runtime
`MapManager` legado para criaturas/controlados y luego el `MoveSpline` canónico del
mapa; si no puede demostrar finalización, rechaza la entrada.

Las regresiones fijan que un teleport pendiente no cambia posición, emote ni
paquetes, y que un spline controlado activo no cambia jugador, tiempo o criatura;
la ruta positiva existente sigue funcionando. Formato, diff, arquitectura,
`cargo check` y `validation-v2 final` pasan en el SHA integrado, con 3.878 tests de
`wow-world` sin fallos. Esto no cierra #63: quedan pasajeros de transportes y
reset, giro de vehículos, movers no Creature, muerte/BG/taxi, ACK/orden y QA viva
con capturas. La visibilidad diferida de `MoveInitActiveMoverComplete` pertenece al
puente ya integrado de #588 y no se duplica.

#### Entrega F1 bajo #63 — membresía de pasajeros de transporte

PR #855 (`10528d45`) completa el siguiente tramo de `MovementHandler.cpp:361-390`.
Antes de publicar el movimiento aceptado, el handler consulta el mapa canónico, retira
el Player del `Transport` anterior cuando cambia o se separa y añade el Player al
`Transport` tipado solo si existe y está en el mundo. El mapa conserva la única
autoridad de la colección de pasajeros; si el objetivo no puede resolverse, la
operación devuelve `ResetTransport` y el estado de sesión/Player se limpia. Las
regresiones cubren attach, switch, detach y objetivo ausente; el movimiento completo
queda en 47/47 y el perfil final pasa con 3.880 tests de `wow-world`. #63 sigue abierto
para seats/turning de vehículos, validación completa de offsets/coordenadas, movers no
Creature, muerte/BG/taxi, ACK/orden y QA viva con capturas.

#### Entrega F1 bajo #63 — ACK `MoveTimeSkipped` del mover controlado

PR #862 (`28762f16`) completa la frontera de `MovementHandler.cpp:721-739` que
seguía admitiendo solo el Player. La admisión compara el GUID con el
`m_unitMovedByMe` activo; el reloj uint32 se incrementa en el Player o en el
Creature/Pet controlado que realmente mueve, y `MoveSkipTime` se publica desde
la posición y el GUID de ese mover, como `mover->SendMessageToSet`. La regresión
controlada prueba el reloj del Creature y la visibilidad del observador. Pasan
los tests enfocados (2), la suite de movimiento (49), arquitectura, `world-server`
y `validation-v2 quick` en el manifiesto registrado. Esta entrega no cierra #63:
quedan ACK/force/knockback/taxi/death/BG restantes, offsets/seats completos,
otros tipos de mover y QA viva con capturas.

#### Entrega F1 bajo #63 — ACK de fuerzas del mover controlado

PR #864 (`2d375ef1`) completa la siguiente frontera de
`MovementHandler.cpp:581-663`: Apply, Remove y mod-magnitude ACK comparan el GUID
del estado con el `m_unitMovedByMe` activo, leen la magnitud esperada del `Unit`
que realmente mueve, ajustan el tiempo aceptado y publican desde su posición/GUID.
La sesión conserva la admisión y libera la consulta de mapa antes de enviar, por lo
que no hay entrega bajo un cerrojo de mapa. La regresión cubre las tres operaciones,
un GUID incorrecto y la ruta de visibilidad; pasan 50 tests de movement handlers,
arquitectura, `world-server` y `validation-v2 quick` con el manifiesto
`target/validation-v2/manifests/20260914T050653.422270Z-262358-quick.json`.
Esta entrega no cierra #63: quedan ACK de velocidad ordinaria, knockback, death/BG/
taxi, offsets/seats completos, otros movers, capturas y QA viva.

#### Entrega F1 bajo #63 — ACK de knockback del mover controlado

PR #866 (`0079daa8`) completa la frontera de admisión de
`MovementHandler.cpp:548-559`. El handler valida el estado con el Player, pero
acepta el GUID cuando coincide con `_player->m_unitMovedByMe`; el estado de
movimiento aceptado sigue escribiéndose en el Player y `MoveUpdateKnockBack` se
publica desde ese origen, como en C++. La regresión de mover controlado prueba la
admisión y el `source_guid` del Player. Pasan el test enfocado, los 50 tests de
movement handlers, arquitectura, `world-server` y `validation-v2 quick` con el
manifiesto `target/validation-v2/manifests/20260914T052813.807323Z-281858-quick.json`.
Esta entrega no cierra #63: quedan ACK de velocidad ordinaria, death/BG/taxi,
offsets/seats completos, otros movers, capturas y QA viva.

#### Entrega F1 bajo #61 — admisión exacta de arma offhand

PR #869 (`abd396a0`) completa la frontera de admisión que faltaba al consumidor
cuerpo a cuerpo. `Unit::DoMeleeAttackIfReady` (`Unit.cpp:2140`) solo entra en la
rama offhand cuando `!IsInFeralForm()` y `haveOffhandWeapon()`; esta última
resuelve `GetWeaponForAttack` (`Unit.cpp:496`, `Player.cpp:9243-9270`). El Player
canónico comprueba ahora el registro de slot, el tipo de inventario de arma y el
objeto Item no roto, y proyecta las formas Cat/Bear/DireBear/GhostWolf
(`Unit.cpp:8807-8812`). Las regresiones cubren ausencia, objeto usable, objeto
roto y forma feral; pasan los 26 escenarios `combat_tick_`, `cargo check` de
`world-server` y el guard de arquitectura. La issue #61 permanece abierta para
productores de aura, cálculo completo, ciclo reversible de equipo, capturas y QA viva.

Qué conservar en cualquier corte P3: residencia/incarnation del Player canónico,
backpressure y cancelación de la tarea de sesión, transferencia entre mapas, descarga
y barreras de apagado, y la invariante de que no se entrega paquete ni comando entre
sesiones sosteniendo un cerrojo de mapa.

Conteo real de tareas en producción: dos bucles al intervalo de mapa —el canónico, que
entra por `MapManager::update`, y el legado de criaturas—, tres tareas periódicas
(ready-check, lista de reinos, keepalive) y una tarea por sesión. El antiguo
`world_update` no existe como productor. `RuntimeTickOwner` elige entre `Session` y
`GlobalLegacy`, con `GlobalLegacy` por defecto en producción.

Macro P3 entregada por esta revisión: **#787**, el contrato de `ProcessingPlace` y
la colocación de la actualización de sesión. P3.1, P3.2, P3.3 y P3.4 son entregas
acotadas integradas bajo #584. P3.3 ejecuta `ProcessRespawns`/`UpdateSpawnGroupConditions`
después del paso de sesiones admitidas y antes de `ObjectUpdater`, con la misma
clave/incarnation del plan para impedir que una sustitución de mapa herede trabajo.
P3.4 conecta ese plan con producción, usa consumidores por GUID solo para objetos
in-world seleccionados y conserva el owner externo de Creature, el bucle de
transportes y la entrega sin guardas. P3.5 resuelve por fuente
`WorldObject::GetGridActivationRange`, con `m_SightDistance` para Creature/Pet
inactivos y visibilidad de mapa para Players/objetos activos. P3.6 añade el radio de
instancia del Player durante una cinemática activa según el cursor canónico; no mueve
el owner de cinemáticas ni crea una búsqueda de FlyByCamera. Su evidencia, límites y
siguiente residuo medido quedan en `session-578-checkpoint.md`; no se crea una issue
por puente o fichero.
**#785 queda cerrada por premisa falsa**, con esta verificación registrada en la issue.

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

#### Continuation: Session time-synchronization state grouping, level 1

The five time-synchronization fields previously stored directly on `WorldSession` are now
grouped in the private `TimeSynchronizationStateLikeCpp` substate in
`crates/wow-world/src/session/time_synchronization.rs`. `WorldSession` remains the single
canonical owner; this is composition only, with no new authority or public path. The
constructor uses the same initial values, including a six-sample clock-delta queue. Driver,
publication, movement, and affected test consumers now access the grouped state; the
test-only clock-delta setter preserves the movement fixture without exposing production
state.

The behavior contract was checked against TrinityCore 3.4.3 at source SHA
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`: `WorldSession.cpp:488-497` (timer phase),
`WorldSession.cpp:1547-1577` (reset, request publication, and movement adjustment), and
`Handlers/MovementHandler.cpp:743-800` (response sampling and clock-delta calculation).
The existing request order, phase timing, sample capacity, latency filtering, and adjustment
threshold remain unchanged. No packet bytes, registration, admission, persistence, or
lifetime contract changed.

Static inventory for this slice: `session/state.rs` decreased from 2,090 to 2,081 lines and
`session/construction.rs` from 1,185 to 1,182; the extracted state type is in a 142-line
module. No tests, builds, formatters, architecture checks, or QA were run under level 1.
The worktree contains unvalidated changes based on base SHA
`9daa13f663bd1e863a3efed06721c3fcb3b6cd66`, with no validated candidate SHA or commit.

#### Continuation: instance-only Session test-fixture grouping, level 1

The detached difficulty preferences and recent-instance map are grouped in
`session/instances/test_fixtures.rs::InstanceTestFixtureLikeCpp`. The fixture preserves
the four `cfg(test)` fields' previous effective visibility and their
`WorldSession::new` defaults. Production continues to read and mutate the canonical
map-owned Player; instance operations, accessors, and their execution order are
unchanged. Direct test-fixture field paths now use the named fixture, while same-named
methods remain method calls.

The ownership references were reviewed against TrinityCore 3.4.3 at pinned source SHA
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`: `Entities/Player/Player.h`'s
`GetDungeonDifficultyID`/`SetDungeonDifficultyID` (1962, 1965),
`GetRaidDifficultyID`/`SetRaidDifficultyID` (1963, 1966),
`GetLegacyRaidDifficultyID`/`SetLegacyRaidDifficultyID` (1964, 1967), and
`GetRecentInstanceId`/`SetRecentInstance`/`m_recentInstances` (2512–2523). This is
ownership context only and makes no new parity claim.

Static source review found four grouped fields and four matching `Default` initializers;
the three difficulty constants and the empty recent-instance map retain their prior
values. No tests, builds, formatters, architecture checks, or QA were run under level 1.
The session syntax-ownership inventory remains a final-acceptance item. The worktree
remains unvalidated at base SHA `9daa13f663bd1e863a3efed06721c3fcb3b6cd66`, with no
validated candidate SHA or commit.

#### Continuation: visibility-only Session test-fixture grouping, level 1

The detached seer and pre-owner phase-shift inputs are grouped in
`session/visibility/test_fixtures.rs::VisibilityTestFixtureLikeCpp`. Both remain
`cfg(test)` state on the single `WorldSession` fixture owner. Production visibility
continues to derive viewpoint and phase from the canonical map-owned Player; no
visibility decision, Player ownership, packet, or lifecycle behavior changed. Existing
visibility-test accessors remain methods, while direct fixture-field consumers now use
the grouped fixture path.

The organization boundary was reviewed against TrinityCore 3.4.3 at pinned source SHA
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`: `Entities/Player/Player.h::m_seer` (2417),
`SetSeer` (2423), `Entities/Player/Player.cpp::UpdateVisibilityForPlayer` (23337), and
`Entities/Object/Object.h::GetPhaseShift` (505–511). This records ownership context only
and makes no new parity claim.

The fixture preserves the prior `WorldSession::new` defaults (`None` and
`PhaseShift::default()`). No tests, builds, formatters, architecture checks, or QA were
run under level 1. The session syntax-ownership inventory remains a final-acceptance
item. The worktree remains unvalidated at base SHA
`9daa13f663bd1e863a3efed06721c3fcb3b6cd66`, with no validated candidate SHA or commit.

#### Continuation: Player item test-fixture grouping, level 1

Fifteen `cfg(test)` Player-item fields now share
`session/player_items/test_fixtures.rs::PlayerItemTestFixtureLikeCpp`: handle-less bank and
inventory capacities, the in-memory inventory/buyback projection, and item-modifier, item-set,
combat-stat, Titan Grip, and average-item-level evidence. `WorldSession` remains the existing
fixture owner; production inventory and item-modifier authority remain on canonical `Player`.
Every member retains its former `pub(in crate::session)` visibility and constructor default.
Existing item/session methods still access this storage in the same Player-item operations;
their signatures and the buyback/packet adapters are unchanged.

The responsibility anchors were checked against TrinityCore 3.4.3 at pinned source SHA
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`: `Entities/Player/Player.cpp::CanStoreNewItem`
(9610), `StoreNewItem` (11166), `_ApplyItemMods` (7654), `_ApplyAllItemMods` (8575),
`UpdateAverageItemLevelTotal` (28803), and `UpdateAverageItemLevelEquipped` (28860). This is
test-fixture storage organization only; item mutation order, modifier recalculation, buyback
state, packets, persistence, and Player ownership are unchanged, with no new parity claim.

Static source review matched all 15 fixture declarations to their 15 `Default` initializers and
found no remaining direct Session field accesses; item accessors and canonical Player inventory
methods remain unchanged. `session/state.rs` decreased from 1,823 to 1,784 lines and
`session/construction.rs` from 1,040 to 1,012; the new fixture is 55 lines. The session
syntax-ownership policy remains unreconciled under level 1 and is a final-acceptance item. No
tests, builds, formatters, architecture checks, or QA were run. The worktree remains unvalidated
at base SHA `9daa13f663bd1e863a3efed06721c3fcb3b6cd66`, with no validated candidate SHA or
commit.

#### Continuation: quest item-mutation persistence organization, level 1

The complete quest-persistence planning block for bank and item mutations moved from
`handlers/quest/persistence.rs` to its private `persistence/item_mutations.rs` child. It
contains the bank-move planner, begin/withdraw/finish item-transfer planners, void-storage
status projection, aggregate transfer planner, and quest-bound source-item objective planner.
Method names, bodies, `pub(crate)` visibility, call order, and the parent persistence module's
remaining quest-status/load operations are unchanged.

The move was checked against the pinned TrinityCore 3.4.3 source at SHA
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`: `Player.cpp::StoreNewItem` (11166),
`MoveItemFromInventory` (11637), `MoveItemToInventory` (11655), `ItemAddedQuestCheck`
(16067), and `ItemRemovedQuestCheck` (16088), plus
`VoidStorageHandler.cpp::HandleVoidStorageTransfer` (90). These anchors establish the
existing item/quest operation context; this structural move makes no new parity claim and
changes no persistence participant, transaction, recovery, or publication behavior.

Before the cut, a static text comparison confirmed that the 434-line source block (including
its separator) matched the child body exactly. Separate comparisons confirmed the retained
prefix and suffix in `persistence.rs` were unchanged. Static inventory: the parent decreased
from 1,017 to 585 lines; `persistence/item_mutations.rs` is 445 lines. No tests, builds,
formatters, architecture checks, or QA were run under level 1. The worktree remains
unvalidated at base SHA `9daa13f663bd1e863a3efed06721c3fcb3b6cd66`, with no validated
candidate SHA or commit.

#### Continuation: handle-less RestMgr test fixture grouping, level 1

The test-only RestMgr state and rest-rate values previously stored as separate
`WorldSession` fields are grouped in `RestMgrTestFixtureLikeCpp`, declared with the
`rest_progression` responsibility. The fixture is available only under `cfg(test)` and is
used when a test session has no canonical Player handle. Production rest state remains
owned by the canonical `Player`; loaded player-flag fixtures remain separate because they
model Player persistence data rather than RestMgr state. Constructor defaults are unchanged.

This is a storage-only fixture refactor checked against TrinityCore 3.4.3 at source SHA
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`: `Entities/Player/RestMgr.cpp:26-30,32-83`
(initialization and XP bonus state), `:95-120,139-159` (rest flags, time, update, and load),
and `Entities/Player/Player.cpp:7298-7405` (area/zone transitions). No runtime phase,
rest-state transition, packet, persistence contract, or public path changed.

Static inventory for this slice: `session/state.rs` decreased from 2,081 to 2,053 lines and
`session/construction.rs` from 1,182 to 1,164. All consumer references were migrated to the
test-only substate by static search. No tests, builds, formatters, architecture checks, or QA
were run under level 1. The worktree contains unvalidated changes based on base SHA
`9daa13f663bd1e863a3efed06721c3fcb3b6cd66`, with no validated candidate SHA or commit.

#### Continuation: Player skill test-fixture grouping, level 1

The six handle-less Player skill fixtures (`player_skill_values`, retained skill rows,
non-durable tombstones, row-load/completeness flags, and occupied-slot count) are grouped in
the test-only `PlayerSkillTestFixtureLikeCpp` under `session/progression`. `WorldSession`
remains the fixture's owner, while production skill authority remains on canonical `Player`.
The constructor's empty maps/sets, false flags, and absent slot count are unchanged; skill
mutations, tombstone persistence handling, and the fallback boundary are unchanged.

The source ownership and persistence contract was checked against TrinityCore 3.4.3 at SHA
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`: `Entities/Player/Player.h:2144-2158,2896-2897`
and `Entities/Player/Player.cpp:5633-5694,20348-20397,25723-25851` cover Player-owned skill
fields, loading, updates, and `_SaveSkills`. No gameplay, transaction, packet, or public
contract changed.

Static inventory for this slice: `session/state.rs` decreased from 2,053 to 2,041 lines and
`session/construction.rs` from 1,164 to 1,156; direct fixture consumers in skill progression,
persistence, and focused scenarios now use the named substate. No tests, builds, formatters,
architecture checks, or QA were run under level 1. The worktree contains unvalidated changes
based on base SHA `9daa13f663bd1e863a3efed06721c3fcb3b6cd66`, with no validated candidate SHA
or commit.

#### Continuation: Player spell and trait test-fixture grouping, level 1

The handle-less test representation of known spells, persisted PlayerSpell rows and their
completeness/fallback metadata, favorites/removals/dependent rows, and TraitConfig/TraitEntry
projections is now grouped in `PlayerSpellAndTraitTestFixtureLikeCpp` under
`session/spell_state`. The fixture is `cfg(test)` only; production spell and trait authority
remains on canonical `Player`. Empty maps/sets, false completeness flags, and the empty
known-spell list preserve the constructor defaults. Snapshot creation, bootstrap, persistence,
and test consumers now read or write the same nested fields; no operation semantics changed.

The ownership and persistence boundary was checked against TrinityCore 3.4.3 at source SHA
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`: `Entities/Player/Player.h:1834-1853,2961`,
`Entities/Player/Player.cpp:18924-18943` (`_LoadSpells`), `:20399-20451` (`_SaveSpells`), and
`:26635-26700` (`_LoadTraits`). This storage-only change does not alter spell/trait load order,
save transactions, packets, or public contracts.

Static inventory for this slice: `session/state.rs` decreased from 2,041 to 1,991 lines and
`session/construction.rs` from 1,156 to 1,132. The state declaration is now below the ordinary
2,000-line review threshold; its acceptance ceiling remains unchanged until final evidence.
Direct fixture consumers were migrated by static search. No tests, builds, formatters,
architecture checks, or QA were run under level 1. The worktree contains unvalidated changes
based on base SHA `9daa13f663bd1e863a3efed06721c3fcb3b6cd66`, with no validated candidate SHA
or commit.

#### Continuation: character account registration organization, level 1

The account-scoped `PacketHandlerEntry` declarations formerly inline in
`handlers/character/account.rs` now live in five private registration children grouped by
character setup, session services, world queries, world services, and inventory actions.
`account.rs` retains its account-session methods and mounts the private `registrations` module;
the registration metadata, handler names, closures, and declaration order are unchanged.

Before removing the original block, a static text comparison of that source span against the
concatenated child bodies reported no differences, and the ordered 70-opcode list matched the
pre-move inventory. The character-family source scan in `handlers/character_tests/loot.rs` now
includes the registration module and all five children. The existing frozen dispatch contract
remains in `session/tests/dispatch.rs::every_registered_opcode_keeps_its_handler_status_and_processing_like_cpp`,
with duplicate registration coverage in
`session/tests/dispatch.rs::dispatch_table_has_no_duplicate_registered_opcodes`; neither test
was changed or run. This is a physical organization change only and introduces no new parity
claim.

Static inventory: `handlers/character/account.rs` decreased from 1,437 to 212 lines; the
registration children are 244, 188, 244, 381, and 178 lines respectively, with a 9-line
private module declaration. No tests, builds, formatters, architecture checks, or QA were run
under level 1. The worktree remains unvalidated at base SHA
`9daa13f663bd1e863a3efed06721c3fcb3b6cd66`, with no validated candidate SHA or commit.

#### Continuation: isolated battle-pet fixture state, level 1

Twenty-two test-only battle-pet fallback and evidence fields are grouped in
`BattlePetTestFixtureLikeCpp` under `session/pets/test_fixtures.rs`. This includes stat-store
injection, trainer-selection overrides, represented pets and slots, critter/query state, and
the associated criteria evidence. Their `WorldSession::new` values are unchanged, including
the three locked-empty slot defaults. The production
`battle_pet_account_attachment_like_cpp` remains a separate canonical field; no production
state, battle-pet operation, or test accessor signature changed. Existing nested-field
visibilities were retained, and the seven direct-consumer files now access the fixture through
the grouped state.

Static inventory: `session/state.rs` decreased from 1,991 to 1,915 lines and
`session/construction.rs` from 1,132 to 1,090; the new test-only fixture module is 99 lines.
The fixture declares and initializes the same 22 members, and `WorldSession::new` now uses
its `Default` implementation. The session syntax-ownership policy was not regenerated or
accepted under level 1; its changed field-path inventory remains an explicit final-acceptance
item. No tests, builds,
formatters, architecture checks, or QA were run. The worktree remains unvalidated at base SHA
`9daa13f663bd1e863a3efed06721c3fcb3b6cd66`, with no validated candidate SHA or commit.

#### Continuation: quest-only Session test-fixture grouping, level 1

Twenty-seven `cfg(test)` fields for handle-less quest status, recurrence, reward side-effect
evidence, titles, and quest sharing are grouped in
`session/quest/test_fixtures.rs::QuestTestFixtureLikeCpp`. `WorldSession` remains the fixture
owner; canonical Player gameplay state, production quest catalogs, quest-point-of-interest data,
XP/discovery state, achievements, instance resets, and area-exploration criteria remain separate.
The fixture preserves each member's prior effective visibility and `WorldSession::new` values.
All Session consumers now use the named fixture path; Player quest snapshots and loot/directory
context fields with overlapping names remain on their own types. Existing Session methods with
the same names remain method calls rather than fixture-field accesses.

The ownership references were reviewed against TrinityCore 3.4.3 at pinned source SHA
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`: `Entities/Player/Player.cpp::RewardQuest` (14625),
`GetQuestStatus` (15532), `SetQuestStatus` (15557), and daily/weekly/monthly/seasonal recurrence
setters (24037, 24061, 24077, 24067); quest-sharing adapters are
`Handlers/QuestHandler.cpp::HandleQuestConfirmAccept` (499), `HandlePushQuestToParty` (603), and
`HandleQuestPushResult` (758). This test-fixture organization changes no quest transition,
reward, packet, persistence, admission, or publication behavior and makes no new parity claim.

Static source review found 27 fixture declarations and 27 matching `Default` initializers,
with no remaining direct `WorldSession` field paths for those members. The moved consumers were
enumerated with `rg -n '\.quest_test_fixture_like_cpp\.' crates/wow-world/src -g '*.rs'`; the
fixture-member/default name comparison returned no diff. `session/state.rs` decreased from 1,915
to 1,823 lines and `session/construction.rs` from 1,090 to 1,040; the new test-only fixture is
105 lines. The session syntax-ownership policy was not regenerated or accepted under level 1;
its changed field-path inventory remains a final-acceptance item. No tests, builds, formatters,
architecture checks, or QA were run. The worktree remains unvalidated at base SHA
`9daa13f663bd1e863a3efed06721c3fcb3b6cd66`, with no validated candidate SHA or commit.

#### Continuation: trade-only Session test-fixture grouping, level 1

The ten detached trade and cancel-evidence fields are grouped in
`session/social/test_fixtures.rs::TradeTestFixtureLikeCpp`. Their prior visibility and
`WorldSession::new` defaults are preserved. Snapshot, mutation, and accessor methods are
unchanged; `Player::TradeData` remains the production authority, and no trade transition,
packet, persistence, or publication behavior changed. Direct fixture accesses now use the
named nested path, while same-named methods remain method calls.

The ownership references were reviewed against TrinityCore 3.4.3 at pinned source SHA
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`: `Entities/Player/Player.h::GetTradeData`
(1449), `Entities/Player/TradeData.h::TradeData` (37–39) and its trade state fields
(76–87), plus `Entities/Player/TradeData.cpp::SetItem` (58), `SetSpell` (84), `SetMoney`
(103), `SetAccepted` (135), and `UpdateServerStateIndex` (150). This is ownership context
only and makes no new parity claim.

Static source review found ten fixture fields and ten matching `Default` initializers,
including the existing slot-count array and client/server index defaults. No tests, builds,
formatters, architecture checks, or QA were run under level 1. The session syntax-ownership
inventory remains a final-acceptance item. The worktree remains unvalidated at base SHA
`9daa13f663bd1e863a3efed06721c3fcb3b6cd66`, with no validated candidate SHA or commit.

#### Continuation: Player bootstrap-catalog test-fixture grouping, level 1

The three `PlayerStart` option mirrors and three Player-create catalog overrides are grouped
in `session/test_support/test_fixtures.rs::PlayerBootstrapCatalogTestFixtureLikeCpp`.
The prior `cfg(test)` visibility and `WorldSession::new` defaults are preserved. Existing
setter/getter methods and the test-only catalog assembler remain unchanged; this moves no
production Player data and changes no creation, spell-learning, reputation, exploration,
packet, or persistence behavior.

The ownership context was reviewed against TrinityCore 3.4.3 at pinned source SHA
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`: `World/World.cpp` loads
`CONFIG_START_ALL_SPELLS`, `CONFIG_START_ALL_EXPLORED`, and `CONFIG_START_ALL_REP`
(1564–1571); `Entities/Player/Player.cpp::Create` (386) consumes the Player-create path
and invokes `LearnDefaultSkills`/`LearnCustomSpells` (506–507), with the latter defined
at 23778. This is organization context only and makes no new parity claim.

Static source review found six fixture fields and six matching `Default` initializers.
No tests, builds, formatters, architecture checks, or QA were run under level 1. The
session syntax-ownership inventory remains a final-acceptance item. The worktree remains
unvalidated at base SHA `9daa13f663bd1e863a3efed06721c3fcb3b6cd66`, with no validated
candidate SHA or commit.

#### Continuation: loaded Player flag test-fixture grouping, level 1

The two loaded Player-flag values and their application marker are grouped in
`session/persistence/test_fixtures.rs::LoadedPlayerFlagsTestFixtureLikeCpp`. Their prior
`cfg(test)` visibility and defaults are preserved. Persistence load/save/commit code,
visibility and rest accessors continue using the same fields through the grouped path;
canonical Player flag storage and application order are unchanged.

The ownership references were reviewed against TrinityCore 3.4.3 at pinned source SHA
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`: `Entities/Player/Player.cpp::LoadFromDB`
(17060), which applies `fields.playerFlags`/`fields.playerFlagsEx` through
`ReplaceAllPlayerFlags`/`ReplaceAllPlayerFlagsEx` (17323–17324), and the canonical
`Player::SetPlayerFlag`/`SetPlayerFlagEx` update-field accessors in `Player.h`
(2672, 2677). This records ownership context only and makes no new parity claim.

Static source review found three fixture fields and three matching `Default` initializers
(`None`, `None`, and `false`). No tests, builds, formatters, architecture checks, or QA
were run under level 1. The session syntax-ownership inventory remains a final-acceptance
item. The worktree remains unvalidated at base SHA
`9daa13f663bd1e863a3efed06721c3fcb3b6cd66`, with no validated candidate SHA or commit.

#### Continuation: support-feature test-fixture grouping, level 1

The five test-only support-system switches are grouped in
`session/support_features/test_fixtures.rs::SupportFeatureTestFixtureLikeCpp`. Their
prior visibility and `WorldSession::new` defaults (`true` for support overall and `false`
for tickets, bugs, complaints, and suggestions) are preserved. Existing accessors and
test policy assembly remain unchanged; this moves no production configuration owner and
does not change feature admission or response behavior.

The configuration ownership context was reviewed against TrinityCore 3.4.3 at pinned
source SHA `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`: `World/World.cpp` reads
`CONFIG_SUPPORT_ENABLED`, `CONFIG_SUPPORT_TICKETS_ENABLED`, `CONFIG_SUPPORT_BUGS_ENABLED`,
`CONFIG_SUPPORT_COMPLAINTS_ENABLED`, and `CONFIG_SUPPORT_SUGGESTIONS_ENABLED` and applies
them to `SupportMgr` (584–595). This is organization context only and makes no new parity
claim.

Static source review found five fixture fields and five matching `Default` initializers.
No tests, builds, formatters, architecture checks, or QA were run under level 1. The
session syntax-ownership inventory remains a final-acceptance item. The worktree remains
unvalidated at base SHA `9daa13f663bd1e863a3efed06721c3fcb3b6cd66`, with no validated
candidate SHA or commit.

#### Continuation: guild-only Session test-fixture grouping, level 1

The four handle-less guild membership, invitation, authority-evidence, and invite-history
fields are grouped in `session/social/test_fixtures.rs::GuildTestFixtureLikeCpp`. Their
prior `cfg(test)` visibility and `WorldSession::new` defaults are preserved. Canonical
Player guild ownership, existing guild methods, and invite handling remain unchanged; no
guild admission, packet, persistence, or publication behavior changed.

The ownership context was reviewed against TrinityCore 3.4.3 at pinned source SHA
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`: `Entities/Player/Player.h::GetGuildId`
(1944), `SetGuildIdInvited` (1943), and `GetGuildIdInvited` (1947), plus
`Guilds/Guild.cpp::HandleInviteMember` (1650–1695). This is structural context only and
makes no new parity claim.

Static source review found four fixture fields and four matching `Default` initializers
(`0`, `false`, `0`, and an empty invite vector). No tests, builds, formatters, architecture
checks, or QA were run under level 1. The session syntax-ownership inventory remains a
final-acceptance item. The worktree remains unvalidated at base SHA
`9daa13f663bd1e863a3efed06721c3fcb3b6cd66`, with no validated candidate SHA or commit.

#### Continuation: calendar-request test-fixture grouping, level 1

Three test-only calendar request evidence vectors are grouped in
`session/social/test_fixtures.rs::CalendarTestFixtureLikeCpp`. Their prior visibility and
empty-vector defaults are preserved. Calendar request recording and accessor methods
retain their existing order and signatures; no event admission, packet, persistence, or
publication behavior changed.

The organization boundary was reviewed against TrinityCore 3.4.3 at pinned source SHA
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`: `Handlers/CalendarHandler.cpp`'s
`HandleCalendarCommunityInvite` (108), `HandleCalendarAddEvent` (114), and
`HandleCalendarRemoveEvent` (222). This is ownership context only and makes no new
parity claim.

Static source review found three fixture fields and three matching `Default`
initializers. No tests, builds, formatters, architecture checks, or QA were run under
level 1. The session syntax-ownership inventory remains a final-acceptance item. The
worktree remains unvalidated at base SHA
`9daa13f663bd1e863a3efed06721c3fcb3b6cd66`, with no validated candidate SHA or commit.

#### Continuation: duel-only Session test-fixture grouping, level 1

Six detached duel and spell-selection evidence fields are grouped in
`session/social/test_fixtures.rs::DuelTestFixtureLikeCpp`. Their prior `cfg(test)`
visibility and defaults are preserved. The canonical Player duel arbiter and all
existing request, accept, cancel, and accessor methods retain their owners and call
paths; no duel admission, state transition, packet, or spell behavior changed.

The ownership context was reviewed against TrinityCore 3.4.3 at pinned source SHA
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`: `Entities/Player/Player.h::SetDuelArbiter`
(1927), and `Handlers/DuelHandler.cpp::HandleCanDuel` (29), `HandleDuelAccepted` (58),
and `HandleDuelCancelled` (86). This is structural context only and makes no new parity
claim.

Static source review found six fixture fields and six matching `Default` initializers
(`None` for the arbiter and empty evidence vectors). No tests, builds, formatters,
architecture checks, or QA were run under level 1. The session syntax-ownership inventory
remains a final-acceptance item. The worktree remains unvalidated at base SHA
`9daa13f663bd1e863a3efed06721c3fcb3b6cd66`, with no validated candidate SHA or commit.

#### Continuation: login-admission read module, level 1

The contiguous login-time battleground-location, homebind-location, and guild-membership
reads are now owned by the private child module
`handlers/character/world_entry/login/admission.rs`. The post-instance login coordinator
still owns phase order, homebind validation/repair, and canonical Player construction.
The helper returns only the values the coordinator already held locally; it adds no
canonical state, persistence operation, or adapter.

The existing Rust request sequence is unchanged: conditional battleground location,
homebind location, then guild membership. Map classification is returned with those
values because the coordinator uses it again later in the same login operation. The
homebind read's existing failure branches still log, kick, and stop login; battleground
and guild outcomes retain their prior warning/unknown behavior. The location/guild
source-order assertion in `character_tests/group.rs` and the whole-character-source
scanner in `character_tests/loot.rs` now include the child module.

Reviewed target-source context at TrinityCore 3.4.3 SHA
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`: `Player.cpp::LoadFromDB` (17060),
including `_LoadHomeBind` (17350) and `_LoadBGData` (17376), and
`CharacterHandler.cpp::HandlePlayerLogin` (1070) with guild-row handling (1124).
This move preserves the existing Rust flow; it does not reorder it to match C++ or
claim a new parity result.

Static inventory: `world_entry/login.rs` decreased from 1,936 to 1,820 lines, and the
new admission child is 163 lines. Existing source assertions were redirected to the
child where they inspect the moved read family. No tests, builds, formatters,
architecture checks, or QA were run under level 1. The worktree remains unvalidated
at base SHA `9daa13f663bd1e863a3efed06721c3fcb3b6cd66`, with no validated candidate
SHA or commit.

#### Continuation: spell target-resolution phase, level 1

The implicit-destination selection and per-effect target-data snapshots moved from the
spell execution coordinator into the private child module
`session/spell_effects/execution/target_resolution.rs`. The coordinator still calls the
phase at the same point - after its represented power-debit branch and before constructing
`SpellGo`. The helper returns the same three local values the coordinator previously
held; it adds no state authority, persistence boundary, or async/lock scope. Later
continuations moved the post-publication phases, the fallback, and cast completion into
their own private children while leaving packet publication and the remaining direct
effect/aura phases in the coordinator.

Effect order, target-A/target-B order, replacement of prior implicit destinations,
per-effect destination snapshots, transport-offset projection, and `SpellGo` map-id
projection are unchanged. Existing movement and nearby-entry scenario coverage remains
registered; no test uses a source-text include of `execution.rs`, so no source scanner
needed relocation.

Reviewed target-source context at TrinityCore 3.4.3 SHA
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`: `SpellCastTargets::ModDst`
(`Spell.cpp:401-412`), `Spell::SelectSpellTargets` (`Spell.cpp:736`), and
`Spell::handle_immediate` target-selection/`SendSpellGo` path (`Spell.cpp:3773-3843`).
This is a structural move only; it does not expand the represented target-selection
coverage or claim parity for the surrounding spell executor.

Static inventory: `session/spell_effects/execution.rs` decreased from 1,257 to 1,197
lines; the new target-resolution child is 102 lines. The coordinator remains above
1,000 lines and retains the rest of the ordered cast operation for a later cohesive
split/review. No tests, builds, formatters, architecture checks, or QA were run under
level 1. The worktree remains unvalidated at base SHA
`9daa13f663bd1e863a3efed06721c3fcb3b6cd66`, with no validated candidate SHA or commit.

#### Continuation: pet login hydration family, level 1

The contiguous pet-login work now lives in the private
`handlers/character/world_entry/login/pet_loading.rs` child. It keeps the existing reset
request, pet-authority load start, stable read, conditional active-pet aura/effect/spell/
cooldown/charge/declined-name reads, and in-memory talent/spec reset in the same Rust
order. The login coordinator calls it at the former block position; group reset and group
membership loading still follow. It uses the same lifecycle port and mutates the same
canonical pet owner through the existing Session operations.

Reviewed target-source context at TrinityCore 3.4.3 SHA
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`: `WorldSession::HandlePlayerLogin`
(`CharacterHandler.cpp:1244-1259`) resets pet spells and specializations before pet
resummoning; `Player::LoadFromDB` calls `_LoadPetStable` (`Player.cpp:17663`); and
`Pet::LoadPetFromDB`'s query-holder callback loads aura/effect rows, then spells and
cooldowns/charges (`Pet.cpp:386-410`). The Rust operation retains its existing serial
lifecycle-port request order and its current placement of the reset around its own stable
and pet-row hydration. This move does not reconcile those broader phase differences or
claim new C++ parity.

The persistence source assertion now scans both the coordinator and pet child, and the
whole-character source inventory includes the child. Static inventory: `world_entry/login.rs`
decreased from 1,820 to 1,543 lines; `world_entry/login/pet_loading.rs` is 297 lines. No
tests, builds, formatters, architecture checks, or QA were run under level 1. The worktree
remains unvalidated at base SHA `9daa13f663bd1e863a3efed06721c3fcb3b6cd66`, with no
validated candidate SHA or commit.

#### Continuation: post-SpellGo target-effect phase grouping, level 1

The post-publication farsight/visibility, teleport/bind, and empty-effects fallback phase
now runs through `execution/post_spell_go.rs::apply_post_spell_go_target_effects_like_cpp`.
The next GameObject summon loop uses the adjacent private
`apply_spell_gameobject_summon_effects_like_cpp` helper; the coordinator retains the
following visibility refresh at its original point. Both helpers are called immediately
after the unchanged `SpellGo` publication. Per-effect target-data lookup, loop order,
fallback construction, result handling, and the existing visibility awaits retain their
prior order. These helpers only receive borrowed spell/target data and the existing
catalog capability; `WorldSession` remains the state owner, with no new persistence,
lock, or runtime boundary.

Reviewed target-source context at TrinityCore 3.4.3 SHA
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`: `Spell::DoProcessTargetContainer`
(`Spell.cpp:3950-3962`) and `SpellEffects.cpp`'s effect-handler table entries for
teleport/bind (93-99), farsight (160), and wild object summon (164), with the corresponding
handlers `EffectTeleportUnits` (938), `EffectAddFarsight` (2237),
`EffectSummonObjectWild` (2937), and `EffectBind` (5213). This is source-context review
for a structural move; it does not claim that the represented execution covers all C++
targeting or handler-mode behavior.

Static inventory: `session/spell_effects/execution.rs` decreased from 1,197 to 1,124
lines; `execution/post_spell_go.rs` is 126 lines. The coordinator remains above 1,000
lines at this checkpoint and still owns the direct-effect match, aura sequence, primary
fallback, and cast-completion phase. No tests, builds, formatters, architecture checks,
or QA were run under level 1. The worktree remains unvalidated at base SHA
`9daa13f663bd1e863a3efed06721c3fcb3b6cd66`, with no validated candidate SHA or commit.

#### Continuation: preparation, fallback, and completion phase grouping, level 1

The pre-publication preparation sequence now runs through the private
`execution/preparation.rs` helper. Cast preparation, the required nearby-entry rejection,
and client power debit remain ordered before implicit-target resolution. Each original
early exit still publishes interrupted frames; the nearby-entry failure still sends the
same cast-failure packet, and the power-failure branch retains its conditional cast-time
restoration. The helper preserves the cast-check result's nested optional focus shape:
`None` represents those same terminal paths to the coordinator, while `Some(None)` still
means preparation succeeded without a resolved focus object. Terminal paths return
without continuing to target resolution or `SpellGo`.

The legacy primary-effect fallback and the final execute-log, threat, and player-cooldown
bookkeeping now live in `primary_effect_fallback.rs` and `completion.rs`. The coordinator
calls them consecutively in their original order after the aura phase. The completion
helper retains the existing difficulty-specific spell data lookup and cooldown-state
gate; neither helper adds a state owner, persistence fence, or publication phase.

Reviewed target-source context at TrinityCore 3.4.3 SHA
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`: `Spell::prepare` (`Spell.cpp:3411`),
`Spell::SelectImplicitNearbyTargets` (`Spell.cpp:1103`), `Spell::TakePower`
(`Spell.cpp:5378`), and `Spell::FinishTargetProcessing` (`Spell.cpp:8493-8496`). This
records the relevant preparation, rejection, debit, and execute-log source context only;
the represented Rust paths retain their existing behavior and coverage boundary.

Static inventory: `session/spell_effects/execution.rs` decreased from 1,124 to 959
lines. The new `preparation.rs`, `primary_effect_fallback.rs`, and `completion.rs` children
are 80, 91, and 75 lines respectively. The coordinator is now below the 1,000-line
ordinary cohesion-review threshold; the larger direct-effect and aura phases remain for
later review. No tests, builds, formatters, architecture checks, or QA were run under
level 1. The worktree remains unvalidated at base SHA
`9daa13f663bd1e863a3efed06721c3fcb3b6cd66`, with no validated candidate SHA or commit.

#### Continuation: persisted-transport login restoration, level 1

The persisted-transport restore and fallback phase now lives in the private
`world_entry/login/transport_restore.rs` child. The coordinator still stores the loaded
identity before this call and performs the same transport restore before its next-level
XP refresh and reputation read. The helper preserves the saved-transport/non-battleground
gate, the existing async transport resolver, successful location and canonical transport
updates, the invalid-transport homebind fallback, and transport clearing when no transport
is eligible. Optional canonical world-map establishment remains conditional on the same
attached-controller value. The helper takes only the existing login inputs and mutable local
map/location values; it introduces no state owner, persistence path, packet, or runtime loop.

Reviewed target-source context at TrinityCore 3.4.3 SHA
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`: `Player::LoadFromDB` (`Player.cpp:17443-17499`)
resolves a saved transport through `CreateMap`/`GetTransport`, checks the expected map,
converts passenger offsets to world coordinates, falls back to homebind for unavailable or
invalid attachments, and adds a successful passenger. This source review does not claim that
the existing Rust resolver/map-install path now has that complete C++ behavior; the change is
a structural move only.

The login source-order test now requires identity setup, transport restoration, and reputation
loading to remain in that order and checks the success, clear, and homebind branches. The
character persistence and whole-character source inventories now include the new child.
Static inventory: `world_entry/login.rs` decreased from 1,543 to 1,502 lines, and
`world_entry/login/transport_restore.rs` is 79 lines. No tests, builds, formatters,
architecture checks, or QA were run under level 1. The worktree remains unvalidated at base
SHA `9daa13f663bd1e863a3efed06721c3fcb3b6cd66`, with no validated candidate SHA or commit.

#### Continuation: CUF profile login hydration, level 1

The CUF profile read now lives in the private `world_entry/login/cuf_profiles.rs` child. The
coordinator retains it after inventory hydration and before the currency request. Within the
helper, represented profiles are cleared before the same lifecycle-port request; loaded rows
are converted field-for-field, passed through the existing profile validator, and the profile
family is marked loaded only after a `Loaded` outcome, including an empty row vector. Failure
still logs after the initial clear, and a mismatched row-family outcome remains unreachable.

Reviewed target-source context at TrinityCore 3.4.3 SHA
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`: the `Player::LoadFromDB` call site is
`Player.cpp:17886`, and `Player::_LoadCUFProfiles` is `Player.cpp:17928-17960`. C++ uses
`id > MAX_CUF_PROFILES` before indexing an array of `MAX_CUF_PROFILES`, while the existing
Rust profile validator rejects `id >= MAX` to avoid that out-of-bounds case. The split keeps
that established Rust behavior and does not claim newly proven CUF parity.

The login source-order test pins inventory → CUF profiles → currencies and checks clear →
request → loaded-marker ordering. The character persistence and whole-character source
inventories now include the child; the existing CUF boundary test still covers rejecting the
legacy out-of-bounds ID. Static inventory: `world_entry/login.rs` decreased from 1,502 to
1,455 lines, and `world_entry/login/cuf_profiles.rs` is 66 lines. No tests, builds,
formatters, architecture checks, or QA were run under level 1. The worktree remains
unvalidated at base SHA `9daa13f663bd1e863a3efed06721c3fcb3b6cd66`, with no validated
candidate SHA or commit.

#### Continuation: canonical currency login hydration, level 1

The character-currency row read and application now lives in the private
`world_entry/login/currency_loading.rs` child. Its boolean result preserves the existing
outer-login early exit when canonical Player currency state is unavailable; adapter failure
still logs and allows login to continue. Loaded rows still require a known currency type,
retain an existing map entry through `or_insert_with`, initialize new entries as unchanged,
and publish through `set_player_currencies_like_cpp`. A mismatched auxiliary result remains
unreachable. The coordinator keeps this call after CUF loading and before the existing spell
read; no canonical currency owner or persistence adapter changed.

Reviewed target-source context at TrinityCore 3.4.3 SHA
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`: `Player::_LoadCurrency`
(`Player.cpp:6771-6798`) skips unknown `CurrencyTypes` rows and inserts unchanged values;
`Player::LoadFromDB` calls it at `Player.cpp:17370`. C++ performs that call materially earlier
than the current Rust login coordinator. This structural move deliberately preserves the
existing Rust phase rather than reconciling login order or claiming currency-load parity.

The login source-order test now pins CUF → currency hydration → spell loading and checks the
canonical owner, store filter, existing-entry preservation, and failure branches. Both the
character persistence scan and whole-character source inventory include the new child. The
existing Session currency tests exercise the canonical currency APIs, but no direct dynamic
scenario currently drives this complete login currency adapter. Static inventory:
`world_entry/login.rs` decreased from 1,455 to 1,414 lines, and
`world_entry/login/currency_loading.rs` is 65 lines. No tests, builds, formatters,
architecture checks, or QA were run under level 1. The worktree remains unvalidated at base
SHA `9daa13f663bd1e863a3efed06721c3fcb3b6cd66`, with no validated candidate SHA or commit.

#### Continuation: action-button login hydration, level 1

Action-button row loading and packet projection now live in the private
`world_entry/login/action_buttons.rs` child. The coordinator still calls it after glyph
hydration and before storing the post-load identity. The helper preserves reset-before-context
lookup, the active-spec/trait-config selection, the lifecycle-port request, row bounds and
positive-action filter, canonical action-button recording, loaded marking only for a `Loaded`
outcome, failure logging, and the same fixed 180-entry packed array returned to the later
login packet phase. Missing specialization state still kicks and terminates login through the
existing call-site return path.

Reviewed target-source context at TrinityCore 3.4.3 SHA
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`: `Player::LoadFromDB` calls
`StartLoadingActionButtons` after inventory (`Player.cpp:17748-17756`);
`Player::StartLoadingActionButtons` selects the active talent group and trait configuration,
issues the asynchronous action query, and checks the live Player GUID in its callback
(`Player.cpp:27033-27075`); `LoadActions` applies rows and sends the action buttons
(`Player.cpp:27077-27082`). This move preserves the current Rust await/result contract and does
not claim that its session-side timing matches the C++ callback/publication path.

The login source-order test pins glyph loading → action-button hydration → identity storage and
checks active configuration, row filtering, canonical recording, marking, and packet encoding.
The character persistence scan and whole-character source inventory include the child. No
dynamic test currently exercises this complete login query/packet phase. Static inventory:
`world_entry/login.rs` decreased from 1,414 to 1,371 lines, and
`world_entry/login/action_buttons.rs` is 67 lines. No tests, builds, formatters, architecture
checks, or QA were run under level 1. The worktree remains unvalidated at base SHA
`9daa13f663bd1e863a3efed06721c3fcb3b6cd66`, with no validated candidate SHA or commit.

#### Continuation: glyph login hydration, level 1

Glyph row loading now lives in the private `world_entry/login/glyph_loading.rs` child. The
coordinator still calls it after spell loading and mount promotion and before action-button
hydration. The helper preserves the represented-glyph reset before the lifecycle-port request,
the per-row GlyphProperties filter through `load_represented_glyph_row_like_cpp`, loaded marking
only for a `Loaded` outcome, and failure logging without aborting login. The
`reputation_rows_complete_like_cpp` flag, previously declared inside the glyph block, stays in
the coordinator at the same point because the later reputation phase owns it.

Reviewed target-source context at TrinityCore 3.4.3 SHA
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`: `Player::LoadFromDB` calls `_LoadGlyphs` after
`_LoadSpells` and the collection loads (`Player.cpp:17710-17717`); `Player::_LoadGlyphs`
(`Player.cpp:26573-26598`) skips talent groups `>= MAX_SPECIALIZATIONS`, slots
`>= MAX_GLYPH_SLOT_INDEX` and ids missing from `sGlyphPropertiesStore`, then calls `SetGlyph`.
This move preserves the existing Rust row validation and phase; it does not claim new parity.

The new login source-order test pins mount promotion → glyph hydration → action-button
hydration, checks that the coordinator no longer issues the glyph request itself, and checks
the reset, request, catalog filter, marking and failure branch in the child. The action-button
order test now anchors on the delegated glyph call. The character persistence scan and
whole-character source inventory include the child. No dynamic test currently exercises this
complete login glyph phase. Static inventory: `world_entry/login.rs` decreased from 1,371 to
1,335 lines, and `world_entry/login/glyph_loading.rs` is 60 lines. No tests, builds,
formatters, architecture checks, or QA were run under level 1. The worktree remains unvalidated
at base SHA `9daa13f663bd1e863a3efed06721c3fcb3b6cd66`, with no validated candidate SHA or commit.

#### Continuation: reputation login hydration, level 1

Reputation row loading now lives in the private `world_entry/login/reputation_loading.rs` child.
The coordinator still calls it after persisted-transport restoration and next-level XP refresh,
and before saved health/power restoration. The helper preserves the lifecycle-port request, the
row conversion into `CharacterReputationRowLikeCpp`, the merge through
`load_character_reputation_rows_like_cpp`, the missing-Faction.db2 warning, and failure logging
without aborting login. It now returns the completion flag instead of mutating a coordinator
local; the coordinator binds `reputation_rows_complete_like_cpp` immutably and the later
complete-spell-rows gate consumes it unchanged. The former `let mut ... = false` declaration
left after the glyph extraction is removed.

Reviewed target-source context at TrinityCore 3.4.3 SHA
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`: `Player::LoadFromDB` calls
`m_reputationMgr->LoadFromDB` at `Player.cpp:17746`; `ReputationMgr::LoadFromDB`
(`ReputationMgr.cpp:729`) runs `Initialize()` and then merges rows whose faction can have
reputation. C++ performs this after inventory/action-button loading; this move preserves the
current Rust phase and does not claim order or reputation-load parity.

The new login source-order test pins transport restoration → reputation loading → saved health
→ the completion-flag consumer, checks that the coordinator no longer issues the request or
declares a mutable flag, and checks the child's request, conversion, merge, warnings and return.
The transport-restore order test now anchors on the delegated call. The character persistence
scan and whole-character source inventory include the child. No dynamic test currently
exercises this complete login reputation phase. Static inventory: `world_entry/login.rs`
decreased from 1,335 to 1,300 lines, and `world_entry/login/reputation_loading.rs` is 60 lines.
No tests, builds, formatters, architecture checks, or QA were run under level 1. The worktree
remains unvalidated at base SHA `9daa13f663bd1e863a3efed06721c3fcb3b6cd66`, with no validated
candidate SHA or commit.

#### Continuation: talent login hydration, level 1

Talent row loading now lives in the private `world_entry/login/talent_loading.rs` child. The
coordinator still calls it after skill loading and skill-rewarded spell changes, and before
`CONFIG_START_ALL_SPELLS` custom spells and the loaded-spell dependency pass. The helper preserves
the represented-talent reset before the lifecycle-port request, the per-row
`load_represented_talent_row_with_spell_side_effects_like_cpp` call against the TalentTab catalog
with the same `known_spells` and `skill_rewarded_dependent_spells` accumulators (now borrowed
`&mut` from the coordinator), loaded marking and completion only for a `Loaded` outcome, and
failure logging without aborting login. It returns the completion flag; the coordinator binds
`talent_rows_complete_like_cpp` immutably and the complete-spell-rows gate consumes it unchanged.

Reviewed target-source context at TrinityCore 3.4.3 SHA
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`: `Player::LoadFromDB` calls `_LoadTalents` before
`_LoadSpells` (`Player.cpp:17709-17710`); `Player::_LoadTalents` (`Player.cpp:26623-26633`) looks
each row up in `sTalentStore` and calls `AddTalent(..., false)`. The Rust coordinator runs this
phase after the spell/skill rows; this move preserves that existing phase and does not claim order
or talent-load parity.

The new login source-order test pins skill loading → talent hydration → custom spells → the
completion-flag consumer, checks that the coordinator no longer issues the request or declares a
mutable flag, and checks the child's reset, request, side-effect call, accumulators, marking,
failure branch and return. The character persistence scan and whole-character source inventory
include the child. No dynamic test currently exercises this complete login talent phase. Static
inventory: `world_entry/login.rs` decreased from 1,300 to 1,259 lines, and
`world_entry/login/talent_loading.rs` is 74 lines. No tests, builds, formatters, architecture
checks, or QA were run under level 1. The worktree remains unvalidated at base SHA
`9daa13f663bd1e863a3efed06721c3fcb3b6cd66`, with no validated candidate SHA or commit.

#### Continuation: persisted skill-row login query, level 1

The persisted `character_skills` query now lives in the private
`world_entry/login/skill_loading.rs` child. It returns the raw represented rows (positive skill
ids, `step` 0, persisted value/max/profession slot, `Unchanged` state) together with the
completion flag. The coordinator keeps the `// ── C++ Player::_LoadSkills ──` phase marker, the
`skill_info_by_id` accumulator, race/class/level normalization through
`loaded_skill_info_like_cpp`, fist-weapon synchronization, the completion-gated canonical
`replace_player_skill_records_like_cpp` call with its kick/return, and the skill-rewarded spell
pass. Only the query/row conversion moved; failure still logs without aborting login and leaves
the canonical owner untouched.

Reviewed target-source context at TrinityCore 3.4.3 SHA
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`: `Player::LoadFromDB` calls `_LoadSkills` at
`Player.cpp:17696`; `Player::_LoadSkills` starts at `Player.cpp:25723`. This move preserves the
existing Rust split between query and normalization and does not claim new skill-load parity.

The new login source-order test pins favorite spells → skill-row query → normalization → the
completion-gated replacement, checks that the coordinator no longer issues the request or
declares a mutable completion flag, and checks the child's request, filter, row shape, failure
branch and return tuple. The persistence scan and whole-character source inventory include the
child. No dynamic test currently exercises this complete login phase. Static inventory:
`world_entry/login.rs` decreased from 1,259 to 1,221 lines, and
`world_entry/login/skill_loading.rs` is 66 lines. No tests, builds, formatters, architecture
checks, or QA were run under level 1. The worktree remains unvalidated at base SHA
`9daa13f663bd1e863a3efed06721c3fcb3b6cd66`, with no validated candidate SHA or commit.

#### Continuation: spell and favorite-spell login queries, level 1

The persisted `character_spell` and favorite-spell queries now live in the private
`world_entry/login/spell_loading.rs` child. `load_character_spell_rows_for_login_like_cpp`
returns a private `LoginSpellRowsLikeCpp` projection with the same three accumulators the
coordinator previously filled inline (raw logical rows with positive `i32` ids and `Unchanged`
state, `AddSpell` side-effect spells via `loaded_spell_for_add_spell_side_effects_like_cpp`, and
active client spells via `active_known_spell_for_send_like_cpp`) plus the completion flag.
`load_character_favorite_spells_for_login_like_cpp` returns the favorite set and its completion
flag. The coordinator destructures both results into the same local names, keeping
`known_spells` and `loaded_spell_side_effect_spells` mutable for the later skill/talent/default
passes, and still declares the skill-rewarded accumulators before `_LoadSkills`. The two row
converters moved out of the coordinator's import list into the child. Failure of either query
still logs without aborting login and leaves the completion flag false for the
complete-spell-rows gate.

Reviewed target-source context at TrinityCore 3.4.3 SHA
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`: `Player::LoadFromDB` calls `_LoadSpells` with the
spell and favorite results (`Player.cpp:17710`); `Player::_LoadSpells` (`Player.cpp:18924-18943`)
calls `AddSpell` per row and then marks favorites only for spells already in `m_spells`. Rust
retains its existing split projection and later favorite application; this move does not claim
new order or spell-load parity.

The new login source-order test pins currency hydration → spell rows → favorites → skill rows →
the complete-spell-rows gate, checks that the coordinator no longer issues either request or
declares mutable completion flags, and checks the child's filters, converters, completion and
failure branches. The currency and skill-row order tests now anchor on the delegated calls. The
persistence scan and whole-character source inventory include the child. No dynamic test
currently exercises this complete login phase. Static inventory: `world_entry/login.rs`
decreased from 1,221 to 1,150 lines, and `world_entry/login/spell_loading.rs` is 121 lines. No
tests, builds, formatters, architecture checks, or QA were run under level 1. The worktree
remains unvalidated at base SHA `9daa13f663bd1e863a3efed06721c3fcb3b6cd66`, with no validated
candidate SHA or commit.

#### Continuation: default-skill login pass, level 1

The `LearnDefaultSkills` representation now lives in the private
`world_entry/login/default_skills.rs` child as the synchronous
`apply_default_skills_for_login_like_cpp`. The coordinator still captures
`persisted_skill_count` after quest loading, calls the child, and then runs the skill-rewarded
spell pass over the returned entries. The helper preserves the three catalog-store guard, the
`default_starting_skill_info_like_cpp` candidates, the skip for skills already held with a
positive value, the 256-slot limit, profession-slot reuse, the Deleted→Changed / otherwise New
state rule, the `skill_info_by_id` insertion, and the canonical
`replace_player_skill_records_like_cpp` call. The former `kick` + `return` becomes `kick` +
`None`, and the coordinator returns on `None` at the same point; `skill_records` and
`skill_info_by_id` are borrowed `&mut` from the coordinator.

Reviewed target-source context at TrinityCore 3.4.3 SHA
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`: `Player::LoadFromDB` calls `LearnDefaultSkills` after
spell and quest loading (`Player.cpp:17737-17740`); `Player::LearnDefaultSkills`
(`Player.cpp:23798-23813`) skips held skills and rows above the player's level, then calls
`LearnDefaultSkill`. This move preserves the existing Rust representation and does not claim new
parity.

The new login source-order test pins quest loading → persisted count → default-skill pass →
skill-rewarded spells, checks that the coordinator no longer computes candidates or declares the
entry vector, and checks the child's filters, limit, state rule, replacement, kick/None path and
return. The persistence scan and whole-character source inventory include the child. No dynamic
test currently exercises this complete login phase. Static inventory: `world_entry/login.rs`
decreased from 1,150 to 1,097 lines, and `world_entry/login/default_skills.rs` is 86 lines. No
tests, builds, formatters, architecture checks, or QA were run under level 1. The worktree remains
unvalidated at base SHA `9daa13f663bd1e863a3efed06721c3fcb3b6cd66`, with no validated candidate
SHA or commit.

#### Continuation: character-aura login hydration, level 1

The character-aura and aura-effect queries now live in the private
`world_entry/login/aura_loading.rs` child. The coordinator still calls it after
account-data loading and before the initial `_ApplyAllItemMods` replay. The former inline block
scope becomes the helper body unchanged: aura authority is reset to incomplete first, both rows
families are converted with `object_guid_from_db_binary_like_cpp`, failures log without aborting
login, `load_represented_character_auras_like_cpp` runs with both row sets, and aura authority
becomes complete only when both queries completed.

Reviewed target-source context at TrinityCore 3.4.3 SHA
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`: `Player::LoadFromDB` calls `_LoadAuras` with the aura
and aura-effect results at `Player.cpp:17718`; `Player::_LoadAuras` starts at `Player.cpp:18034`.
This move preserves the existing Rust phase and does not claim new aura-load parity.

The new login source-order test pins account data → aura hydration → initial item mods, checks
that the coordinator no longer issues either request, and checks the child's reset → apply →
authority order, converters and failure branch. The persistence scan and whole-character source
inventory include the child. No dynamic test currently exercises this complete login phase. Static
inventory: `world_entry/login.rs` decreased from 1,097 to 1,016 lines, and
`world_entry/login/aura_loading.rs` is 99 lines. No tests, builds, formatters, architecture checks,
or QA were run under level 1. The worktree remains unvalidated at base SHA
`9daa13f663bd1e863a3efed06721c3fcb3b6cd66`, with no validated candidate SHA or commit.

#### Continuation: PlayerSpellMap login finalization, level 1

The merge of raw `character_spell` rows with the canonical known-spell projection now lives in
the private `world_entry/login/spell_map_finalization.rs` child as the synchronous
`finalize_player_spell_map_for_login_like_cpp`. The coordinator still runs it after mount
promotion and before the `LearnDefaultSkills` summary log. The coordinator now computes the
unchanged five-flag conjunction (spell rows, favorites, talents, account mounts, reputation) into
`login_spell_map_authority_complete_like_cpp` and passes it with the raw rows (moved), favorites and
the skill-rewarded dependent/removed sets (borrowed). The helper keeps the dependent/favorite
derivation, Removed-state rewrite, active refresh for enabled rows, canonical-spell insertion,
`set_complete_represented_player_spell_rows_like_cpp`, acquisition-snapshot marking and both
warnings unchanged. Evaluating the conjunction before the call is order-neutral: every operand is
an immutable bool already bound.

No new C++ anchor: this is Rust representation of the post-`_LoadSpells` PlayerSpellMap and claims
no new parity. The new login source-order test pins mount promotion → gate → finalization →
summary, checks all five gate operands remain, and checks the child's merge rules, gate and
warnings. The spell-phase order test now anchors on the gate binding. The persistence scan and
whole-character source inventory include the child. Static inventory: `world_entry/login.rs`
decreased from 1,016 to 957 lines, and `world_entry/login/spell_map_finalization.rs` is 91 lines.
No tests, builds, formatters, architecture checks, or QA were run under level 1. The worktree
remains unvalidated at base SHA `9daa13f663bd1e863a3efed06721c3fcb3b6cd66`, with no validated
candidate SHA or commit.

#### Continuation: group-membership login hydration, level 1

The group-membership query now lives in the private `world_entry/login/group_loading.rs` child.
The coordinator still calls it after the login pet-state load and before the next-level XP
refresh/clamp. The helper preserves clearing the owned group first, the lifecycle-port request,
restoration of only the first row through `load_represented_group_by_db_store_id_like_cpp`
followed by `reset_group_update_sequence_if_needed_like_cpp`, and the failure warning without
aborting login.

Reviewed target-source context at TrinityCore 3.4.3 SHA
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`: `Player::LoadFromDB` calls `_LoadGroup` at
`Player.cpp:17368`; `Player::_LoadGroup` (`Player.cpp:18981-19003`) also sets the leader flag,
subgroup, party type and offline difficulty changes. Rust applies the leader flag later
(`apply_represented_group_leader_flag_like_cpp`); this move preserves that existing split and does
not claim new group-load parity.

The new login source-order test pins pet state → group membership → XP clamp, checks the
coordinator no longer issues the request or clears the group itself, and checks the child's
reset → request order, first-row restore, sequence reset and failure warning. The persistence scan
and whole-character source inventory include the child. Static inventory: `world_entry/login.rs`
decreased from 957 to 936 lines, and `world_entry/login/group_loading.rs` is 41 lines. No tests,
builds, formatters, architecture checks, or QA were run under level 1. The worktree remains
unvalidated at base SHA `9daa13f663bd1e863a3efed06721c3fcb3b6cd66`, with no validated candidate
SHA or commit.

#### Continuation: Player mail login hydration, level 1

The mail query and canonical mail-owner replacement now live in the private
`world_entry/login/mail_loading.rs` child. The coordinator still calls it right after
`ensure_login_player_controller_like_cpp` and before played-time/money hydration. The helper
preserves the lifecycle-port request, the kick on a failed or mismatched outcome, the
`PlayerMailRecord` conversion (zero template id → `None`), and the kick when
`replace_owned_player_mails_like_cpp` fails. Each former `kick` + `return` becomes `kick` +
`return false`; the coordinator returns on `false` at the same point, so login still aborts.

Reviewed target-source context at TrinityCore 3.4.3 SHA
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`: `Player::LoadFromDB` calls `_LoadMail` with mail and
mail-item results at `Player.cpp:17759`; `Player::_LoadMail` starts at `Player.cpp:18560`. C++ loads
mail much later than the current Rust coordinator; this move preserves the Rust phase and does not
claim order or mail-load parity.

The new login source-order test pins controller → mail → money hydration, checks the coordinator
no longer issues the request or replaces mails itself, and checks the child's failure kicks,
conversion and replacement. The persistence scan and whole-character source inventory include the
child. Static inventory: `world_entry/login.rs` decreased from 936 to 904 lines, and
`world_entry/login/mail_loading.rs` is 58 lines. No tests, builds, formatters, architecture checks,
or QA were run under level 1. The worktree remains unvalidated at base SHA
`9daa13f663bd1e863a3efed06721c3fcb3b6cd66`, with no validated candidate SHA or commit.
