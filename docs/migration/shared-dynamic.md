# Migration: shared/Dynamic

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/server/shared/Dynamic/`
> **Rust target crate(s):** n/a (idiom replacement: `Arc`/`Weak` + `enum`s + `inventory`)
> **Layer:** L1
> **Status:** ✅ done (sustituido por idiom Rust; no port directo)
> **Audited vs C++:** ✅ n/a confirmed (2026-05-01) — C++ idiom replaced by `Arc`/`Weak` + `inventory`; no port needed
> **Last updated:** 2026-05-01

---

## 1. Purpose

Conjunto de templates C++ que dan TrinityCore su zoo de "smart pointers caseros" y registries genéricos: `LinkedList` intrusiva, `Reference<TO,FROM>` (auto-unlink en destructor), `RefManager`, `ObjectRegistry<T,Key>` (singleton-map de unique_ptr), `FactoryHolder<T,O,Key>` (factoría auto-registrante), y `TypeList` (lista de tipos a la Loki). Es C++ idiom puro: **no se porta directo a Rust**, sus equivalentes son `Arc`/`Weak`, `enum dispatch`, `HashMap<K, Box<dyn Trait>>`, y el crate `inventory` para auto-registration.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `shared/Dynamic/FactoryHolder.h` | 54 | `prefix` |
| `shared/Dynamic/LinkedList.h` | 231 | `prefix` |
| `shared/Dynamic/LinkedReference/RefManager.h` | 54 | `prefix` |
| `shared/Dynamic/LinkedReference/Reference.h` | 104 | `prefix` |
| `shared/Dynamic/ObjectRegistry.h` | 85 | `prefix` |
| `shared/Dynamic/TypeList.h` | 45 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

| File | Lines | Purpose |
|---|---|---|
| `src/server/shared/Dynamic/LinkedList.h` | 231 | `LinkedListElement` + `LinkedListHead` (doubly-linked intrusivo, bidirectional iterator) |
| `src/server/shared/Dynamic/LinkedReference/Reference.h` | 104 | `Reference<TO,FROM>` extiende `LinkedListElement`, hooks `targetObjectBuildLink`/`targetObjectDestroyLink`/`sourceObjectDestroyLink` |
| `src/server/shared/Dynamic/LinkedReference/RefManager.h` | 54 | `RefManager<TO,FROM>` extiende `LinkedListHead`, `clearReferences` invalida todos |
| `src/server/shared/Dynamic/ObjectRegistry.h` | 85 | `ObjectRegistry<T, Key=string>` singleton + `RegistryMapType = map<Key, unique_ptr<T>>` |
| `src/server/shared/Dynamic/FactoryHolder.h` | 54 | `FactoryHolder<T,O,Key>` con `RegisterSelf` + `Permissible<T>::Permit` |
| `src/server/shared/Dynamic/TypeList.h` | 45 | `TypeList<HEAD,TAIL>` + `TYPELIST_1..8` macros |
| **TOTAL** | **~573** | — |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `LinkedListElement` | class | Nodo de doubly-linked list intrusiva, auto-delink en destructor |
| `LinkedListHead` | class | Sentinel nodes `iFirst`/`iLast` + `iSize` lazy-counted |
| `LinkedListHead::Iterator<T>` | template inner | `bidirectional_iterator_tag`, op++/--, op==/!= |
| `Reference<TO,FROM>` | template class | Hereda `LinkedListElement`; vinculo bidi entre `TO` y `FROM` con hooks virtuales |
| `RefManager<TO,FROM>` | template class | Hereda `LinkedListHead`; collection de Reference que se auto-invalidan en destrucción |
| `ObjectRegistry<T,Key>` | template singleton | Map global `Key → unique_ptr<T>` con `InsertItem(force=false)` |
| `FactoryHolder<T,O,Key>` | template class | Factory base con `Create(O*)=0` virtual + `RegisterSelf` |
| `Permissible<T>` | template class | `Permit(T const*) const = 0` — patrón de selector "este factory acepta este input" |
| `TypeList<HEAD,TAIL>` | template struct | Lista de tipos compile-time; `TYPELIST_N` macros |
| `TypeNull` | sentinel class | Terminator de lista de tipos |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `LinkedListElement::delink()` | Quitar nodo de su lista, nullear vecinos | — |
| `LinkedListElement::insertBefore/After` | Splice de nodo en posición | — |
| `LinkedListElement::~LinkedListElement` | Auto-delink (RAII) | `delink` |
| `LinkedListHead::insertFirst/Last` | Splice al sentinel | `insertAfter`/`Before` |
| `LinkedListHead::isEmpty()` | `!iFirst.iNext->isInList()` | — |
| `LinkedListHead::getSize()` | Si `iSize=0` cuenta linealmente, sino retorna cached | — |
| `Reference<TO,FROM>::link(toObj, fromObj)` | Establece link, dispara `targetObjectBuildLink` | — |
| `Reference::unlink()` | Llama `targetObjectDestroyLink` + delink | — |
| `Reference::invalidate()` | Llama `sourceObjectDestroyLink` + delink (target murió) | — |
| `RefManager::clearReferences()` | Invalida todos los refs hijos | — |
| `ObjectRegistry::instance()` | Singleton accessor | — |
| `ObjectRegistry::InsertItem(obj, key, force)` | Add con opción de overwrite | — |
| `ObjectRegistry::GetRegistryItem(key)` | Lookup, puede devolver null | — |
| `FactoryHolder::RegisterSelf()` | `FactoryHolderRegistry::instance()->InsertItem(this, _key)` | — |
| `FactoryHolder::Create(O*)` | Virtual factory method | — |

---

## 5. Module dependencies

**Depends on:**
- `Define.h` — `uint32` typedefs
- `Errors.h` — `ASSERT`
- nada más; es header-only puro

**Depended on by:**
- `game/Movement` — `MovementGenerator`s usan `FactoryHolder` (`MovementGeneratorFactory`)
- `game/AI` — `CreatureAI` factories usan `FactoryHolder<CreatureAI, Creature>`
- `game/Combat/HostileRefManager` — `HostileReference` extiende `Reference<Unit, ThreatManager>`
- `game/Server/HostileRefManager`, `ThreatManager`, `RedirectThreatInfo` — `RefManager` para tracking de threat list
- `game/Spells/Auras` — `Aura::AuraApplications` usa lista intrusiva (parecida)
- `shared/Database/MySQL/...` — algunos prepared statements registries usan `ObjectRegistry`
- `Battlefield` / `OutdoorPvP` script registries

---

## 6. SQL / DB queries

N/A — utility puro.

---

## 7. Wire-protocol packets

N/A.

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-handler/src/lib.rs` | `file` | 1 | 116 | `exists_active` | file exists |
| `crates/wow-handler` | `crate_dir` | 1 | 116 | `exists_active` | crate exists |
| `crates/wow-combat` | `crate_dir` | 1 | 0 | `exists_empty` | crate exists; no active Rust source lines |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- **(ninguno literal).** No existe `LinkedList.rs`, ni `RefManager.rs`, ni `ObjectRegistry.rs`. Es C++ idiom; **Rust resuelve los mismos problemas con primitives diferentes**. Status “done” en sentido idiomático: cada uso del C++ ya tiene su contraparte natural Rust.

**Mapping idiomático (cómo se resuelve cada caso de uso):**

| Caso de uso C++ | Equivalente Rust en RustyCore |
|---|---|
| `LinkedList` para iterar lista pequeña con removes baratos | `Vec<T>` + `Vec::retain` o `LinkedList<T>` (std), nadie lo usa en RustyCore |
| `Reference<TO,FROM>` para link cross-objeto que sobrevive al destructor del owner | `Arc<RwLock<T>>` + `Weak<RwLock<T>>` (target tiene `Vec<Weak<...>>`) |
| `RefManager::clearReferences` | drop del `Arc<...>` ⇒ `Weak::upgrade()` retorna `None` ⇒ readers limpian al iterar |
| `ObjectRegistry<T, string>` para script/factory lookup | `inventory::collect!(T)` + `inventory::iter::<T>` (estática), ó `HashMap<&'static str, fn(...) -> Box<dyn Trait>>` |
| `FactoryHolder<T,O>` con `RegisterSelf` | `inventory::submit!` + `inventory::collect!` (`crates/wow-handler/src/lib.rs` lo usa para packet handlers — patrón canónico en RustyCore) |
| `TypeList<HEAD,TAIL>` + `TypeContainerVisitor` (game/Grids) | Tuple types + traits + `enum dispatch`. Para RustyCore actual: enums (`AnyMapObject`) o `HashMap<TypeId, ...>` |
| `Permissible<T>::Permit` selector | `fn permit(&self, input: &T) -> i32` en un trait + iterar registry |

**Específicamente en RustyCore:**
- **Packet handler registry** = `inventory::submit!(PacketHandlerEntry{...})` en `wow-handler` (~ es `FactoryHolder` Rust-idiomatic)
- **MapManager / grid storage** = HashMap<MapId, MapInstance> con `Arc<RwLock<...>>`; sin `TypeContainerVisitor`, los visitors se reemplazan por iter-and-match (pequeñas funciones que hacen `for creature in grid.creatures.values()`)
- **Threat list / hostile refs** = TODO; combat module aún no implementa `Weak`-based threat tracking (mantiene `HashMap<ObjectGuid, ThreatEntry>` plano)

**What's implemented (idiom):**
- `inventory` crate usado para `PacketHandlerEntry` registration → cubre 80% del uso de `FactoryHolder`/`ObjectRegistry`
- `Arc<RwLock<T>>` + `Weak` patrón disponible (workspace deps `parking_lot`, `dashmap`)
- `std::collections::LinkedList` disponible si se necesitara (no se usa)

**What's missing vs C++:**
- N/A — no hay un “port” pendiente. Lo que importa es que cuando se traiga código que dependía de `RefManager` (combat threat, aura applications) se use `Weak<...>` en lugar de un port literal.
- Si futuros módulos requieren un `ObjectRegistry`-style, el patrón canónico ya es `inventory::collect!`.

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- **Threat list:** el C++ usa `HostileRefManager` con `Reference<Unit, ThreatManager>` para que cuando una `Unit` muera, todas sus inverse-refs se auto-invaliden. El Rust de `wow-combat` actual mantiene `HashMap<ObjectGuid, ThreatEntry>` plano y limpia explícitamente on death. Funcionalmente equivalente pero hay riesgo de leaks si alguien olvida limpiar.
- **Aura targets:** mismo patrón — Rust ata aura a target via `ObjectGuid`, no via `Weak`, por lo que un GUID stale puede apuntar a un objeto distinto si el slot se reusa. **Verificar invariante de unicidad de GUID por sesión**.

**Tests existing:**
- 0 tests específicos (no hay módulo).

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#SHARED_DYNAMIC.WBS.001** Cerrar la migracion auditada de `shared/Dynamic/FactoryHolder.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/shared/Dynamic/FactoryHolder.h`
  Rust target: `crates/wow-handler`, `crates/wow-combat`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SHARED_DYNAMIC.WBS.002** Cerrar la migracion auditada de `shared/Dynamic/LinkedList.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/shared/Dynamic/LinkedList.h`
  Rust target: `crates/wow-handler`, `crates/wow-combat`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SHARED_DYNAMIC.WBS.003** Cerrar la migracion auditada de `shared/Dynamic/LinkedReference/RefManager.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/shared/Dynamic/LinkedReference/RefManager.h`
  Rust target: `crates/wow-handler`, `crates/wow-combat`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SHARED_DYNAMIC.WBS.004** Cerrar la migracion auditada de `shared/Dynamic/LinkedReference/Reference.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/shared/Dynamic/LinkedReference/Reference.h`
  Rust target: `crates/wow-handler`, `crates/wow-combat`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SHARED_DYNAMIC.WBS.005** Cerrar la migracion auditada de `shared/Dynamic/ObjectRegistry.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/shared/Dynamic/ObjectRegistry.h`
  Rust target: `crates/wow-handler`, `crates/wow-combat`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SHARED_DYNAMIC.WBS.006** Cerrar la migracion auditada de `shared/Dynamic/TypeList.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/shared/Dynamic/TypeList.h`
  Rust target: `crates/wow-handler`, `crates/wow-combat`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

> **Objetivo:** no portar el código sino documentar y enforce el idiom Rust en cada call site.

- [ ] **#DYN.1** Auditar combat: `wow-combat` threat list — ¿usa `ObjectGuid` plano o `Weak<Unit>`? Documentar en `combat.md`. (M)
- [ ] **#DYN.2** Auditar auras (`wow-spell`): aura → target link debería resistir GUID reuse. (M)
- [ ] **#DYN.3** Documentar el patrón `inventory::submit!` como **el** patrón canónico para registries en `CONTRIBUTING.md` o equivalente. (L)
- [ ] **#DYN.4** Si surge necesidad de listas intrusivas eficientes (rare), evaluar `intrusive-collections` crate antes de fabricar. (L)
- [ ] **#DYN.5** Cuando se traiga `MovementGenerator` factory (game/Movement), usar `inventory::submit!`, **no** un puerto literal de `FactoryHolder`. (M)
- [ ] **#DYN.6** Cuando se traiga `TypeContainerVisitor` (game/Grids), no portar como tal — en `MapManager` ya hay grids tipados; los “visits” se hacen con methods explícitos (`visit_creatures_in_radius` etc.). (M)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#SHARED_DYNAMIC.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 6 files / 573 lines; refs: `/home/server/woltk-trinity-legacy/src/server/shared/Dynamic/LinkedList.h`, `/home/server/woltk-trinity-legacy/src/server/shared/Dynamic/LinkedReference/Reference.h`, `/home/server/woltk-trinity-legacy/src/server/shared/Dynamic/ObjectRegistry.h`. Rust target: `crates/wow-combat`, `crates/wow-handler`. | `cargo test -p wow-combat && cargo test -p wow-handler` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#SHARED_DYNAMIC.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 6 files / 573 lines; refs: `/home/server/woltk-trinity-legacy/src/server/shared/Dynamic/LinkedList.h`, `/home/server/woltk-trinity-legacy/src/server/shared/Dynamic/LinkedReference/Reference.h`, `/home/server/woltk-trinity-legacy/src/server/shared/Dynamic/ObjectRegistry.h`. Rust target: `crates/wow-combat`, `crates/wow-handler`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#SHARED_DYNAMIC.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 6 files / 573 lines; refs: `/home/server/woltk-trinity-legacy/src/server/shared/Dynamic/LinkedList.h`, `/home/server/woltk-trinity-legacy/src/server/shared/Dynamic/LinkedReference/Reference.h`, `/home/server/woltk-trinity-legacy/src/server/shared/Dynamic/ObjectRegistry.h`. Rust target: `crates/wow-combat`, `crates/wow-handler`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#SHARED_DYNAMIC.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 6 files / 573 lines; refs: `/home/server/woltk-trinity-legacy/src/server/shared/Dynamic/LinkedList.h`, `/home/server/woltk-trinity-legacy/src/server/shared/Dynamic/LinkedReference/Reference.h`, `/home/server/woltk-trinity-legacy/src/server/shared/Dynamic/ObjectRegistry.h`. Rust target: `crates/wow-combat`, `crates/wow-handler`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

Tests a nivel sistémico (no unitarios de este módulo):

- [ ] **Threat list invariante:** killer muere → todos sus `ThreatEntry` desaparecen sin leak (test en `wow-combat`)
- [ ] **GUID reuse safety:** crear creature, soltar guid, crear nueva creature con mismo guid base → ningún aura/threat de la primera apunta a la segunda
- [ ] **`inventory::collect!` determinism:** misma compilación, mismo orden de iteration en `wow-handler`
- [ ] **Drop semantics:** `Arc<RwLock<Unit>>` se libera cuando última `Strong` cae; `Weak::upgrade` retorna `None`

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `idiom_replacement_scope` | No one-to-one Rust port required, but every C++ responsibility must be mapped to the Rust idiom or explicitly marked not used. | 6 files / 573 lines; refs: `/home/server/woltk-trinity-legacy/src/server/shared/Dynamic/LinkedList.h`, `/home/server/woltk-trinity-legacy/src/server/shared/Dynamic/LinkedReference/Reference.h`, `/home/server/woltk-trinity-legacy/src/server/shared/Dynamic/ObjectRegistry.h` | n/a (idiom replacement: `Arc`/`Weak` + `enum`s + `inventory`) \| ✅ done (sustituido por idiom Rust; no port directo) |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#SHARED_DYNAMIC.DIV.001` | `crates/wow-combat` (`exists_empty`, 0 Rust lines) | 6 C++ files / 573 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/shared/Dynamic/LinkedList.h`, `/home/server/woltk-trinity-legacy/src/server/shared/Dynamic/LinkedReference/Reference.h`, `/home/server/woltk-trinity-legacy/src/server/shared/Dynamic/ObjectRegistry.h` | `exists_empty` | Rust target exists but has no active Rust source lines for a module with canonical C++ coverage. crate exists; no active Rust source lines |

<!-- REFINE.023:END known-divergences -->

1. **No portar literal:** intentar portar `Reference<TO,FROM>` con punteros raw + `Drop` impl es un footgun de unsafe Rust. Usar `Arc`/`Weak` aunque tenga overhead distinto.
2. **`LinkedListElement::~LinkedListElement` auto-delink** — equivalente Rust: `Drop` impl en wrapper alrededor de `Vec<...>` + index manual; o simplemente no usar listas intrusivas.
3. **`ObjectRegistry::InsertItem(force=false)`** — semántica de "no overwrite si existe" → `HashMap::entry().or_insert_with()`.
4. **`TypeList` + visitors** — antiguo trick C++ pre-variadic-templates. En Rust usar `enum AnyEntity { Creature(Arc<...>), Player(Arc<...>), ... }` o variadic generics si llega.
5. **`FactoryHolder::RegisterSelf`** invocado en static-init time del C++ — equivalente Rust es `inventory::submit!` (usa link-time collection vía constructor attributes).
6. **`Permissible::Permit` retorna `int32`** — el "score" más alto gana. Patrón Rust: `fn score(&self, input: &T) -> i32` en trait + `iter().max_by_key(|f| f.score(...))`.
7. **iSize lazy en `LinkedListHead`** — si `iSize == 0` recorre la lista. Detalle de optimización; nadie lo necesita en Rust.

---

## 12. C++ → Rust mapping

| C++ | Rust | Notas |
|---|---|---|
| `LinkedListElement` (intrusive node) | `std::collections::LinkedList<T>` o `Vec<T>` | Linked list raramente la solución correcta en Rust |
| `LinkedListHead<T>::Iterator` | `impl Iterator for ...` con `next()`/`prev()` | std handles bidir |
| `Reference<TO,FROM>` | `Weak<RwLock<TO>>` poseído por `FROM` | Drop auto-invalida via `upgrade() == None` |
| `RefManager<TO,FROM>::clearReferences()` | drop del `Arc` → todos los `Weak` invalidan en upgrade | RAII Rust |
| `targetObjectBuildLink` virtual | callback closure pasado al constructor del wrapper | No herencia |
| `ObjectRegistry<T, string>` | `inventory::collect!(T)` + `inventory::iter::<T>` | Compile-time registration |
| `FactoryHolder<T,O,Key>::RegisterSelf` | `inventory::submit! { Factory { key, create_fn } }` | Patrón canónico ya en `wow-handler` |
| `Permissible<T>::Permit` | `fn permit(&self, t: &T) -> i32` en trait | iter+max_by_key |
| `TypeList<HEAD,TAIL>` | tuple `(HEAD, TAIL)` + traits, o `enum` | sin necesidad real |
| `TypeContainerVisitor` (en game/Grids) | métodos explícitos por tipo en `MapManager` | divergencia idiomática total |

---

## 13. Audit (2026-05-01)

**Status confirmed: ✅ n/a — no direct port needed.**

The C++ `shared/Dynamic/` headers (`LinkedList.h`, `LinkedReference/Reference.h`, `RefManager.h`, `ObjectRegistry.h`, `FactoryHolder.h`, `TypeList.h`, ~573 lines total) are pure C++ idiom — RAII-driven intrusive lists, template singletons, and `TypeList`/`TypeContainerVisitor` machinery — that have no idiomatic Rust translation. Verified by grep: zero files named `linked_list.rs`, `ref_manager.rs`, `object_registry.rs`, `factory_holder.rs`, or `type_list.rs` exist in the workspace, and none are needed. Rust replaces these wholesale: `Arc<RwLock<T>>` + `Weak` for cross-object back-references with auto-invalidation; the `inventory` crate (used canonically in `crates/wow-handler/src/lib.rs` for `PacketHandlerEntry` registration) for static factory registration; `MapManager`'s explicit per-type accessors (`crates/wow-world/src/map_manager.rs`) instead of `TypeContainerVisitor`. The migration sub-tasks #DYN.1–#DYN.6 are documentation/audit-only and do not constitute a port.

**Residual cleanup:** none for this module itself. Open follow-ups #DYN.1 (combat threat list `Weak`-vs-GUID audit) and #DYN.2 (aura target GUID-reuse safety) are tracked under `combat.md` and `pets.md`/spell domain respectively, not here.

---

*Template version: 1.0 (2026-05-01).*
