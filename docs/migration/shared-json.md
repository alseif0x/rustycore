# Migration: shared/JSON

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/server/shared/JSON/`
> **Rust target crate(s):** n/a (idiom replacement: `serde` + `serde_json`)
> **Layer:** L1
> **Status:** ✅ done (sustituido por `serde_json` + structs `#[derive(Serialize)]`; no port directo)
> **Audited vs C++:** ✅ n/a confirmed (2026-05-01) — by-design divergence (RapidJSON+protobuf → serde_json+ad-hoc structs); golden-test gaps tracked
> **Last updated:** 2026-05-01

---

## 1. Purpose

Wrapper sobre **RapidJSON** que serializa/deserializa **mensajes Protobuf** a/desde JSON, con campo extra de control: respeta enum names, omite default values, mapea camelCase. En TrinityCore se usa exclusivamente para los payloads de `RealmList` (proto definidos en `proto/Client/RealmList.proto`) que se envuelven en `JamJSONRealmEntry:` / `JSONRealmListUpdates:` / `JSONRealmListServerIPAddresses:` y luego se comprimen con zlib. En RustyCore se sustituye por `serde_json` directo sobre structs Rust nativos (no proto-derived).

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `shared/JSON/ProtobufJSON.cpp` | 457 | `prefix` |
| `shared/JSON/ProtobufJSON.h` | 38 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

| File | Lines | Purpose |
|---|---|---|
| `src/server/shared/JSON/ProtobufJSON.h` | 38 | Two-line interface: `Serialize(Message&)` / `Deserialize(string, Message*)` |
| `src/server/shared/JSON/ProtobufJSON.cpp` | 457 | RapidJSON visitor que recorre `google::protobuf::Reflection` y emite/parsea JSON, manejando todos los `FieldDescriptor::CppType` (INT32, INT64, UINT32, UINT64, DOUBLE, FLOAT, BOOL, ENUM, STRING, MESSAGE) + `FieldDescriptor::TYPE_BYTES` (base64) + repeated fields |
| **TOTAL** | **~495** | — |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `JSON::Serialize(Message const&)` | free fn | Convierte proto → `std::string` JSON |
| `JSON::Deserialize(string, Message*)` | free fn | Convierte JSON → proto, retorna `bool` ok |
| `internal::SerializeMessage` (anon) | recursive helper | Recorre `Message::GetReflection()`, emite JSON via `rapidjson::Writer<StringBuffer>` |
| `internal::DeserializeMessage` (anon) | recursive helper | Parse `rapidjson::Document` → llena proto via `Reflection::SetXxx` |

No hay enums públicos. La complejidad está en switch sobre `CppType` y `Type` (BYTES → base64).

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `JSON::Serialize(message)` | Punto de entrada serialize | `internal::SerializeMessage` recursivo |
| `JSON::Deserialize(json, message*)` | Punto de entrada parse | `rapidjson::Document::Parse`, `internal::DeserializeMessage` |

(`internal::*` no son API pública; son detalles del .cpp.)

---

## 5. Module dependencies

**Depends on:**
- **RapidJSON** (header-only) — `Document`, `Writer<StringBuffer>`, `Value`
- **Google Protobuf** — `Message`, `Descriptor`, `FieldDescriptor`, `Reflection`, `EnumValueDescriptor`
- `Errors.h` — `ASSERT`
- `Util.h` — base64 encode/decode (para `TYPE_BYTES`)

**Depended on by:**
- `shared/Realm/RealmList.cpp` — `JSON::Serialize(RealmListUpdates)` etc.
- `bnetserver/Services` — algunos RPC handlers que devuelven proto JSON

---

## 6. SQL / DB queries

N/A.

---

## 7. Wire-protocol packets

N/A directo. Indirectamente: el output JSON va dentro de payloads bnet auth, no es un opcode WoW.

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/bnet-server/src/realm/mod.rs` | `file` | 1 | 392 | `exists_active` | file exists |
| `crates/wow-proto` | `crate_dir` | 2 | 254 | `exists_active` | crate exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- **(ningún módulo dedicado).** Sustituido por `serde_json` (workspace dep) + structs `#[derive(Serialize, Deserialize)]` per-uso.

**Uso real en RustyCore:**
- `crates/bnet-server/src/realm/mod.rs` — define `RealmListUpdates`, `RealmEntry`, `ClientVersion`, `RealmCharacterCountList`, `RealmListServerIpAddresses`, `AddressFamily`, `IpAddress` como structs Rust con `#[derive(Serialize)]` y atributo `#[serde(rename_all = "camelCase")]`. Llama `serde_json::to_string(&struct).unwrap_or_default()` y compone envelope `format!("...:{}\0", json)`.
- No hay deserialize-side: el bnet-server no necesita parsear JSON entrante (los clientes mandan proto binario, no JSON).

**Por qué no se porta literalmente:**
- TrinityCore JSON-encodea **proto messages**. RustyCore **no usa Protobuf para RealmList JSON** — directamente usa structs Rust serializables. Esto evita la necesidad de un visitor que recorra reflection de `prost`-generated types (técnicamente posible con `prost-reflect` pero overkill para 5 mensajes).
- `prost` (workspace dep en `wow-proto/`) genera `Serialize` derives opcionales si se habilita feature, pero RustyCore optó por structs ad-hoc → menos deps, output controlado.

**What's implemented:**
- Serialización serde de los 7 message types necesarios para realm list
- Envelope strings (`JSONRealmListUpdates:`, `JamJSONRealmEntry:`, `JSONRealmListServerIPAddresses:`, `JSONRealmCharacterCountList:`)
- Trailing `\0` para matchear longitud del C++ (`json.length() + 1`)
- Compresión zlib via `flate2::ZlibEncoder` con prefix `u32` little-endian de tamaño descomprimido

**What's missing vs C++:**
- **Deserialize path:** TrinityCore JSON parser puede leer `JoinRealmRequest` etc. desde JSON; RustyCore no implementa el reverso. Si en el futuro algún cliente/admin tool manda JSON al bnetserver, falta este lado.
- **Generic over proto:** la versión Rust solo serializa los structs que tiene. Para añadir un nuevo proto JSON-able toca añadir struct espejo a mano.
- **Bytes (base64):** ningún campo actual es `bytes`; si se añade hay que confirmar que `serde_json` lo encodea base64 (por defecto `Vec<u8>` se serializa como array JSON; hace falta `#[serde(with = "base64")]` o equivalente).
- **Enum names vs values:** `serde_json` por defecto serializa enum variants como strings (con tag); Protobuf JSON puede emitir como int o name según opciones. Verificar si algún campo es proto enum.

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- **camelCase conversion:** `#[serde(rename_all = "camelCase")]` produce `wowRealmAddress` desde `wow_realm_address`. ProtobufJSON canonical produce el mismo formato → OK en teoría, pero **verificar** con un payload real bnet vs un client capture.
- **Default value omission:** ProtobufJSON omite campos default (proto3 semantic). `serde_json` los incluye — un `population_state: 0` puede salir explícito en RustyCore donde C++ lo omitiría. **El cliente WoW puede ser estricto con esto**.
- **Field ordering:** RapidJSON respeta orden de declaración del proto; serde respeta orden de campos en el struct. Si no coinciden, **el JSON será diferente en orden de keys** (no semánticamente, pero diff-tools y caches sí lo notan).
- **`flate2` default level:** `Compression::default()` = level 6. zlib C++ `compress()` también level 6 por defecto → idéntico output bit-a-bit (probable).

**Tests existing:**
- 0 tests de serialización JSON. **Crítico** dado que el output va por wire al cliente WoW.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#SHARED_JSON.WBS.001** Cerrar la migracion auditada de `shared/JSON/ProtobufJSON.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/shared/JSON/ProtobufJSON.cpp`
  Rust target: `crates/wow-proto`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SHARED_JSON.WBS.002** Cerrar la migracion auditada de `shared/JSON/ProtobufJSON.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/shared/JSON/ProtobufJSON.h`
  Rust target: `crates/wow-proto`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

- [ ] **#JSON.1** Test golden: capturar payload bnet de TrinityCore real (`tcpdump`/Wireshark del JoinRealm response), descomprimir zlib, comparar string-to-string con output de RustyCore para misma input. (M)
- [ ] **#JSON.2** Auditar omisión de default values: `serde_json` con `#[serde(skip_serializing_if = "is_default")]` per-field si C++ lo omite. (M)
- [ ] **#JSON.3** Auditar field ordering: añadir test `serde_json::to_string` produce keys en orden esperado por cliente. (L)
- [ ] **#JSON.4** Si `bytes` fields aparecen, añadir `#[serde(with = "base64")]` con crate `base64` o `serde_bytes`. (L on demand)
- [ ] **#JSON.5** Test round-trip: `to_string` + `from_str` del mismo struct produce equality (sanity de serde derives). (L)
- [ ] **#JSON.6** Si llega caso de necesitar JSON↔Protobuf reflection-driven, evaluar `prost-reflect` antes de mantener structs ad-hoc. (M, on demand)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#SHARED_JSON.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 2 files / 495 lines; refs: `/home/server/woltk-trinity-legacy/src/server/shared/JSON/ProtobufJSON.cpp`, `/home/server/woltk-trinity-legacy/src/server/shared/JSON/ProtobufJSON.h`. Rust target: `crates/wow-proto`. | `cargo test -p wow-proto` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#SHARED_JSON.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 2 files / 495 lines; refs: `/home/server/woltk-trinity-legacy/src/server/shared/JSON/ProtobufJSON.cpp`, `/home/server/woltk-trinity-legacy/src/server/shared/JSON/ProtobufJSON.h`. Rust target: `crates/wow-proto`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#SHARED_JSON.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 2 files / 495 lines; refs: `/home/server/woltk-trinity-legacy/src/server/shared/JSON/ProtobufJSON.cpp`, `/home/server/woltk-trinity-legacy/src/server/shared/JSON/ProtobufJSON.h`. Rust target: `crates/wow-proto`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#SHARED_JSON.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 2 files / 495 lines; refs: `/home/server/woltk-trinity-legacy/src/server/shared/JSON/ProtobufJSON.cpp`, `/home/server/woltk-trinity-legacy/src/server/shared/JSON/ProtobufJSON.h`. Rust target: `crates/wow-proto`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Output `JSONRealmListUpdates:{...}\0` byte-identical (o JSON-equal) al de TrinityCore para mismo realm input
- [ ] zlib payload con prefijo `u32` LE coincide en bytes con C++ `compress()` (level 6)
- [ ] Cliente WoW 3.4.3.54261 acepta el payload y muestra realm list correctamente (test integración)
- [ ] camelCase: `wow_realm_address: 1` → `"wowRealmAddress": 1` en JSON
- [ ] No infinite recursion en serialization (sanity)
- [ ] UTF-8 en `name` con caracteres no ASCII se preserva

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `idiom_replacement_scope` | No one-to-one Rust port required, but every C++ responsibility must be mapped to the Rust idiom or explicitly marked not used. | 2 files / 495 lines; refs: `/home/server/woltk-trinity-legacy/src/server/shared/JSON/ProtobufJSON.cpp`, `/home/server/woltk-trinity-legacy/src/server/shared/JSON/ProtobufJSON.h` | n/a (idiom replacement: `serde` + `serde_json`) \| ✅ done (sustituido por `serde_json` + structs `#[derive(Serialize)]`; no port directo) |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#SHARED_JSON.DIV.001` | _none generated_ | 2 C++ files / 495 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/shared/JSON/ProtobufJSON.cpp`, `/home/server/woltk-trinity-legacy/src/server/shared/JSON/ProtobufJSON.h` | `no_generated_divergence` | No structural divergence found by target-existence scan; this is not a functional audit. |

<!-- REFINE.023:END known-divergences -->

1. **TrinityCore usa proto JSON, RustyCore usa serde:** divergencia consciente. Equivalencia debe **validarse contra payloads reales**, no asumirse.
2. **Trailing `\0`:** importante. C++ incluye el terminator en `length + 1` para zlib. Rust replica con `format!(..."\0")`. Si se omite, el cliente puede leer mal el final del JSON.
3. **Default values:** la diferencia más probable de causar bugs. Proto3 → "no campo" === "campo con default"; el cliente WoW puede tratarlos distinto.
4. **`serde_json::to_string` no garantiza key order estable** entre versiones de la crate (hoy sí, mañana podría no). Si el cliente cachea por hash del JSON o requiere orden, fijar con `BTreeMap` o serializer custom.
5. **`UpdateObject` y otros packets son binarios, no JSON.** JSON se usa **solo** para realm-list payloads en bnet auth — no confundir con el wire protocol del world server.
6. **RapidJSON no sanea inputs:** si un realm name contiene `"` injection-able, se escapa con `\"`. `serde_json` también escapa correctamente — OK.
7. **Performance:** RapidJSON es más rápido que serde_json en C++ benchmarks. Para 5-50 realms a refresh interval de minutos, irrelevante.

---

## 12. C++ → Rust mapping

| C++ | Rust | Notas |
|---|---|---|
| `JSON::Serialize(google::protobuf::Message const&)` | `serde_json::to_string(&MyStruct)` | Per-tipo, no genérico sobre proto |
| `JSON::Deserialize(json, Message*) -> bool` | `serde_json::from_str::<MyStruct>(s) -> Result<...>` | `Result` en lugar de bool |
| RapidJSON `Writer<StringBuffer>` | `serde_json::Serializer` | Hidden por `to_string` |
| RapidJSON `Document::Parse` | `serde_json::from_str` | — |
| Proto `Reflection::Get<Type>` | derive macro `Serialize` | Compile-time vs runtime |
| `FieldDescriptor::CppType` switch | tipos Rust nativos en struct | sin reflection |
| Proto `TYPE_BYTES` → base64 | `#[serde(with = "base64")]` | Si se necesita |
| Proto enum → name string | `#[derive(Serialize)] enum X { ... }` o `serde_repr` para int | Configurable per-enum |
| `bool Deserialize` retorna false on error | `Result<T, serde_json::Error>` | Más expresivo |

---

## 13. Audit (2026-05-01)

**Status confirmed: ✅ n/a — by-design divergence.**

C++ `shared/JSON/ProtobufJSON.{h,cpp}` (~495 lines) is a RapidJSON visitor over `google::protobuf::Reflection` — generic over any proto `Message`. RustyCore deliberately does **not** port this: the only consumer is the bnet realm-list payload, and `crates/bnet-server/src/realm/mod.rs` already provides idiomatic Rust structs with `#[derive(Serialize)]` + `#[serde(rename_all = "camelCase")]` driving `serde_json::to_string`, plus envelope strings (`JSONRealmListUpdates:`, `JamJSONRealmEntry:`, etc.), trailing `\0`, and zlib via `flate2`. Reusing `prost-reflect` for a generic visitor would be overkill for ~7 message types and would add a runtime-reflection dependency not otherwise needed. The divergence is intentional: structs ad-hoc over proto-derived JSON.

**Residual cleanup:** the divergence has known **observable risks** that should not block the n/a status but must be tracked: (a) proto3 default-value omission (RapidJSON omits, `serde_json` emits — possibly visible to strict clients), (b) field key ordering, (c) `bytes` → base64 encoding if any `Vec<u8>` field is added. These are captured as open sub-tasks #JSON.1 (golden test against captured TC payload), #JSON.2 (default omission audit), #JSON.3 (key-order test), and #JSON.4 (base64 on demand). Until #JSON.1 has been executed at least once, treat the wire equivalence as **assumed, not proven**.

---

*Template version: 1.0 (2026-05-01).*
