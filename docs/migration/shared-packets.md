# Migration: shared/Packets

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/server/shared/Packets/`
> **Rust target crate(s):** `crates/wow-packet/`
> **Layer:** L0
> **Status:** ⚠️ partial
> **Audited vs C++:** ⚠️ (see §13)
> **Last updated:** 2026-05-01

---

## 1. Purpose

Infraestructura de serialización/deserialización de paquetes binarios del protocolo WoW 3.4.3. Provee `ByteBuffer` (contenedor genérico para read/write de primitivos con bit-packing) y `WorldPacket` (encapsula opcode + payload). Todos los ~560 handlers de sesión dependen de él.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `shared/Packets/ByteBuffer.cpp` | 207 | `prefix` |
| `shared/Packets/ByteBuffer.h` | 666 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

| File | Lines (aprox) | Purpose |
|---|---|---|
| `src/server/shared/Packets/ByteBuffer.h` | ~800 | Clase ByteBuffer: read<T>/write<T>, bit-packing, posición lectura/escritura, excepciones |
| `src/server/shared/Packets/ByteBuffer.cpp` | ~300 | Conversión endian, manejo excepciones, métodos string |
| `src/server/shared/Packets/MessageBuffer.h` | ~150 | Ring buffer pooling para I/O |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `ByteBuffer` | class | Contenedor con read/write tipado + bit-packing |
| `ByteBufferException` | class | Base excepción |
| `ByteBufferPositionException` | class | Read/write past end |
| `ByteBufferInvalidValueException` | class | Conversión tipo inválida |
| `WorldPacket` | class | Packet con opcode (u16 LE) + payload ByteBuffer |
| `MessageBuffer` | class | Ring buffer I/O |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `ByteBuffer::append<T>(T)` | Añade primitivo (u8/16/32/64, i*, float, double) | `EndianConvert`, `append(uint8*, size)` |
| `ByteBuffer::read<T>()` | Lee primitivo; avanza `_rpos` | `ResetBitPos`, conversión endian |
| `ByteBuffer::WriteBit(bool)` | Escribe 1 bit en `_curbitval`; auto-flush si lleno | `_curbitval`, `_bitpos` |
| `ByteBuffer::ReadBit()` | Lee 1 bit; recarga `_curbitval` | — |
| `ByteBuffer::WriteBits(value, n)` | Escribe N bits | loop WriteBit |
| `ByteBuffer::ReadBits(n)` | Lee N bits | loop ReadBit |
| `ByteBuffer::FlushBits()` | Finaliza bit-pack actual; escribe byte pendiente | `append(uint8*)` |
| `ByteBuffer::ResetBitPos()` | Reset bit pos pre-read byte | `_bitpos = 8` |
| `ByteBuffer::rpos()` / `wpos()` | Get/set read/write position | — |
| `ByteBuffer::bitwpos()` | Get bit position | — |
| `ByteBuffer::PutBits(pos, n, val)` | Reemplaza bits en posición (patch) | — |
| `ByteBuffer::operator<<(T)` / `operator>>(T&)` | Append/read con operadores | — |
| `ByteBuffer::ReadString()` / `ReadCString()` | Lee string null-terminated | — |
| `ByteBuffer::ReadPackedGuid()` / `WritePackedGuid()` | Compression GUID variable-length WoLK | — |

---

## 5. Module dependencies

**Depends on:**
- `shared/Define.h` (uint8/16/32/64, TC_SHARED_API)
- `shared/ByteConverter.h` (`EndianConvert` little-endian)
- STL: `std::vector<uint8>`, `std::string`, `std::array`

**Depended on by:**
- **Todos los ~560 packet handlers** (`WorldSession::HandleXxx`)
- `game/Server/Protocol/Opcodes.h` — enum opcodes en primer u16
- `game/World/WorldSession.h` — constructor WorldPacket
- Todos los crates Rust de packets

---

## 6. SQL / DB queries

N/A — serialización pura, sin queries.

---

## 7. Wire-protocol packets

Define el formato genérico:

```text
[Header]
  u16 LE  — Opcode
[Payload]
  variable — formato específico por opcode
```

Bit-packing: `WriteBits` puede mezclarse con bytes planos en mismo packet; `FlushBits()` obligatorio antes de cambiar modo.

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-packet` | `crate_dir` | 25 | 13058 | `exists_active` | crate exists |
| `crates/wow-packet/src/world_packet.rs` | `file` | 1 | 673 | `exists_active` | file exists |
| `crates/wow-packet/src/header.rs` | `file` | 1 | 100 | `exists_active` | file exists |
| `crates/wow-packet/src/compression.rs` | `file` | 1 | 424 | `exists_active` | file exists |
| `crates/wow-packet/src/lib.rs` | `file` | 1 | 48 | `exists_active` | file exists |
| `crates/wow-packet/src/packets/mod.rs` | `file` | 1 | 26 | `exists_active` | file exists |
| `crates/wow-packet/tests` | `path` | 0 | 0 | `missing_declared_path` | declared/proposed target does not exist |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- `crates/wow-packet/src/world_packet.rs` — ~400 líneas (`WorldPacket` con `BitBuf` state)
- `crates/wow-packet/src/header.rs` — `PacketHeader` (16 bytes: size + GCM tag)
- `crates/wow-packet/src/compression.rs` — Compresión > 1024 bytes
- `crates/wow-packet/src/lib.rs` — Traits `ClientPacket` y `ServerPacket`
- `crates/wow-packet/src/packets/mod.rs` — Inventario módulos por dominio (aura, auth, battlenet, character, chat, combat, gossip, inspect, item, loot, misc, movement, query, quest, party, social, spell, trainer, update)

**What's implemented:**
- `new_empty()`, `new_server()`, `new_client()` — construcción
- `read<T>()`, `write<T>()` — primitivos (u8/u16/u32/u64, i*, f32/f64)
- `read_string()`, `write_string()` — null-terminated
- `read_bits(n)`, `write_bits(val, n)` — bit-packing
- `flush_bits()` — finalizar
- Traits `ClientPacket` / `ServerPacket`
- ~50+ packet types definidos por dominio

**What's missing vs C++:**
- Hierarquía completa de excepciones — Rust usa `Result<T, PacketError>` (idiom diferente, OK)
- `ReadPackedGuid` / `WritePackedGuid` — compresión GUID variable-length (algunos packets sí lo tienen, otros no — auditar)
- Variantes de string (`ReadCString` con trim) — Rust usa `read_string` genérico
- `append_bytes` / `read_bytes` — copiar bloques sin conversión

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- **Bit-packing order:** C++ `_bitpos` decrece (8→0), Rust `wbit_pos` incrementa (0→8). Lógica inversible pero requiere auditoría explícita.
- **String encoding:** WoLK 3.4.3 inconsistente: algunos opcodes null-terminated, otros length-prefixed. Auditoría caso por caso.
- **Float encoding:** ¿endian igual en todos los casos?

**Tests existing:**
- ~10 tests primitivos en `crates/wow-packet/tests/`
- Necesitan tests bit-packing (sequences read_bits/write_bits cross-impl)

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#SHARED_PACKETS.WBS.001** Cerrar la migracion auditada de `shared/Packets/ByteBuffer.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/shared/Packets/ByteBuffer.cpp`
  Rust target: `crates/wow-packet`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SHARED_PACKETS.WBS.002** Partir y cerrar la migracion auditada de `shared/Packets/ByteBuffer.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/shared/Packets/ByteBuffer.h`
  Rust target: `crates/wow-packet`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `needs_split`; C++ file has 666 lines; split by public API, state model, persistence, runtime behavior and tests before implementation. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

- [ ] **#PACKETS.1** Verificar bit-packing: secuencia C++ → leer en Rust → comparar resultado byte-a-byte. (H)
- [ ] **#PACKETS.2** Implementar `ReadPackedGuid` / `WritePackedGuid` (variable-length GUID compression). (M)
- [ ] **#PACKETS.3** Auditar string encoding por opcode (null-term vs length-prefix). (H, ~30+ opcodes)
- [ ] **#PACKETS.4** Implementar `append_bytes` / `read_bytes` (copiar sin conversión tipos). (L)
- [ ] **#PACKETS.5** Garantizar `PacketError` cubre casos `ByteBufferException` C++. (L)
- [ ] **#PACKETS.6** Benchmark serialización Rust vs C++ en packet típico 512 bytes. (M)
- [ ] **#PACKETS.7** Auditar IEEE-754 float encoding (corner cases NaN, Inf, -0). (L)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#SHARED_PACKETS.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 2 files / 873 lines; refs: `/home/server/woltk-trinity-legacy/src/server/shared/Packets/ByteBuffer.h`, `/home/server/woltk-trinity-legacy/src/server/shared/Packets/ByteBuffer.cpp`. Rust target: `crates/wow-packet`. | `cargo test -p wow-packet` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#SHARED_PACKETS.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 2 files / 873 lines; refs: `/home/server/woltk-trinity-legacy/src/server/shared/Packets/ByteBuffer.h`, `/home/server/woltk-trinity-legacy/src/server/shared/Packets/ByteBuffer.cpp`. Rust target: `crates/wow-packet`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#SHARED_PACKETS.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 2 files / 873 lines; refs: `/home/server/woltk-trinity-legacy/src/server/shared/Packets/ByteBuffer.h`, `/home/server/woltk-trinity-legacy/src/server/shared/Packets/ByteBuffer.cpp`. Rust target: `crates/wow-packet`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#SHARED_PACKETS.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 2 files / 873 lines; refs: `/home/server/woltk-trinity-legacy/src/server/shared/Packets/ByteBuffer.h`, `/home/server/woltk-trinity-legacy/src/server/shared/Packets/ByteBuffer.cpp`. Rust target: `crates/wow-packet`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] Serializar+deserializar primitivo u32 → resultado ≡ original
- [ ] Bit-packing: write 4+3+2 bits = 1 byte 0xAB, releer y verificar
- [ ] String null-terminated: write "hello\0", read, comparar
- [ ] Read past end → `Err(PacketError::ReadPastEnd)`
- [ ] Float IEEE-754: write f32(3.14), read, comparar con tolerancia
- [ ] PackedGuid (cuando se implemente): bytes esperados para GUID 0x0150000000ABCDEF
- [ ] Mixed mode: bytes plain + bits + bytes plain con FlushBits intermedio

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 2 files / 873 lines; refs: `/home/server/woltk-trinity-legacy/src/server/shared/Packets/ByteBuffer.h`, `/home/server/woltk-trinity-legacy/src/server/shared/Packets/ByteBuffer.cpp` | `crates/wow-packet/` \| ⚠️ partial |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#SHARED_PACKETS.DIV.001` | `crates/wow-packet/tests` (`missing_declared_path`, 0 Rust lines) | 2 C++ files / 873 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/shared/Packets/ByteBuffer.h`, `/home/server/woltk-trinity-legacy/src/server/shared/Packets/ByteBuffer.cpp` | `missing_declared_path` | Declared/proposed Rust target is absent while C++ coverage exists. declared/proposed target does not exist |

<!-- REFINE.023:END known-divergences -->

- **Bit-packing en WoLK 3.4.3 es delicado:** algunos opcodes mezclan bytes planos + bit-packed. Llamar `FlushBits()` explícito antes de cambiar modo.
- **Endianness:** little-endian en mayoría; algunos casos especiales (GUIDs comprimidos). Ver `ByteConverter.h` C++.
- **`MessageBuffer`:** C++ tiene pooling. Rust usa `BytesMut` directamente con capacidad pre-asignada.
- **Default size:** C++ `DEFAULT_SIZE = 0x1000` (4 KB). Rust 64 bytes, crece dinámico. Puede afectar perf packets grandes.
- **Position rewind:** `rpos(new)` salta. Cuidado: NO resetea estado bits.
- **ASSERT:** C++ excepción on read past end. Rust `Result`. Handlers deben manejar `Err`.

---

## 12. C++ → Rust mapping

| C++ | Rust | Notas |
|---|---|---|
| `ByteBuffer` | `WorldPacket` (en `world_packet.rs`) | `BytesMut` + estados bit |
| `uint8/16/32/64` | `u8/u16/u32/u64` | Conversión endian integrada |
| `float` / `double` | `f32` / `f64` | IEEE-754, conversión endian |
| `std::string_view` | `&str` o `String` | Bytes + null term |
| `WriteBit(bool)` | `write_bit(bool)` | Mismo state |
| `ReadBit()` | `read_bit()` | Mismo state |
| `WriteBits(v, n)` | `write_bits(v, n)` | Loop |
| `ReadBits(n)` | `read_bits(n)` | Loop, retorna u32 |
| `FlushBits()` | `flush_bits()` | Finaliza bit-pack |
| `ResetBitPos()` | `reset_bit_pos()` | — |
| `rpos()` / `wpos()` | `read_pos()` / `write_pos()` | — |
| `ByteBufferException` family | `Result<T, PacketError>` | Match patterns |

---

*Template version: 1.0 (2026-05-01).*

---

## 13. Audit (2026-05-01)

### 13.1 Summary

Side-by-side audit of Rust `WorldPacket` (`crates/wow-packet/src/world_packet.rs`,
`header.rs`, `lib.rs`) vs C++ canonical `ByteBuffer` / `MessageBuffer`
(`src/server/shared/Packets/ByteBuffer.{h,cpp}`,
`src/common/Utilities/MessageBuffer.h`).

The previous status `✅ done` listed in §8 ("What's implemented") is **not
justified**. The Rust side is functionally close on the hot path
(primitives, bit-pack roundtrip, packed GUID 128-bit), but it is
**missing several ByteBuffer features that handlers can legitimately
need** (`PutBits`, `bitwpos`, `WriteString` no-null, `append_pack_xyz`,
`f64`/double, NaN/Inf rejection on float read, bool read normalization
via `read<char> > 0`). Bit-packing direction is **bit-equivalent on the
wire** despite the inverted counter (proof in §13.4) — that hypothesis
in §8 turned out to be a non-issue.

Net: the layer is **safe to keep using** for the opcodes already
shipped, but the public-API gap will keep biting future handler
work, and three of the gaps (PutBits, NaN guard, length-prefix string
helper) actually do affect wire protocol.

### 13.2 Method-by-method table

| C++ method | Rust equivalent | Status | Divergence |
|---|---|---|---|
| `ByteBuffer::append<T>(T)` | `write_uint8/16/32/64`, `write_int8/16/32/64`, `write_float`, `write_bytes` | ✅ | LE conversion explicit via `put_u*_le`; semantically equivalent. No generic `append<T>` — each primitive has its own fn. |
| `ByteBuffer::read<T>()` | `read_uint8/16/32/64`, `read_int…`, `read_float`, `read_bytes` | ✅ | All return `Result<T, PacketError>` instead of throwing. **No `read<double>` / `f64`** — see §13.3-A. |
| `ByteBuffer::WriteBit(bool)` | `write_bit(bool)` | ✅ wire-equivalent | Counter direction inverted (C++ `--_bitpos` from 8→0, Rust `++wbit_pos` from 0→8). Trace in §13.4 proves identical byte output. |
| `ByteBuffer::ReadBit()` | `read_bit()` → `Result<bool>` | ✅ | Same MSB-first ordering. C++ uses `_bitpos` 0..7 with `(byte >> (7-_bitpos)) & 1`; Rust pre-decrements then `(byte >> bit_pos) & 1`. Equivalent. |
| `ByteBuffer::WriteBits(value, n)` | `write_bits(u32, u32)` | ⚠️ | **Type narrowed**: C++ takes `std::size_t` (64-bit), Rust takes `u32`. Survey of callers (`grep write_bits`) shows max width 32 bits, all values ≤ 32-bit, so currently safe — but a future 36-bit field would silently truncate. |
| `ByteBuffer::ReadBits(n)` | `read_bits(u32) -> Result<u32>` | ✅ | Returns u32, matches C++. |
| `ByteBuffer::FlushBits()` | `flush_bits()` | ✅ | C++ no-ops when `_bitpos == 8`; Rust no-ops when `wbit_pos == 0`. Same. |
| `ByteBuffer::ResetBitPos()` | `reset_bits()` (public) + `reset_bit_reader()` (private, auto-called before byte reads) | ✅ | C++ resets only when `_bitpos > 7` (i.e. only the read-side state); Rust splits write/read state and the public `reset_bits()` resets only the reader. Note: `WorldPacket` does **not** auto-flush writes before a non-bit write the way C++ `append()` does (`append()` calls `FlushBits()` at line ByteBuffer.cpp:100). Rust *does* flush in every `write_uint*` etc. — see world_packet.rs:286-329. ✅ semantics-equivalent. |
| `ByteBuffer::HasUnfinishedBitPack()` | — | ❌ missing | Rare; no current caller in Rust. Add if a port uses it. |
| `ByteBuffer::rpos()` / `rpos(n)` | `read_position()` / `reset_read()` | ⚠️ | Rust has a getter and a full-reset, but **no arbitrary `rpos(n)` setter**. C++ uses this to rewind/seek inside packets. |
| `ByteBuffer::wpos()` / `wpos(n)` | — | ❌ missing | No way to query or set the write position. Required by C++ `AppendPackedUInt64` pattern (peek pos, write placeholder, fill in mask later). The Rust `write_packed_guid` works because it computes the mask up-front, but other patch-style writes can't be ported. |
| `ByteBuffer::bitwpos()` / `bitwpos(n)` | — | ❌ missing | Required as the position argument to `PutBits` (see next row). |
| `ByteBuffer::PutBits(pos, value, bits)` | — | ❌ missing | **Used in TC for movement-update field counts that are written before the actual count is known.** Without this, any handler that needs to back-patch a bit-packed length has to be rewritten in two-pass collect-then-write style. Sub-task #PACKETS.8. |
| `ByteBuffer::put<T>(pos, value)` | — | ❌ missing | Byte-level patch; less critical, but same family as `PutBits`. |
| `ByteBuffer::operator<<(T)` / `operator>>(T&)` | each `write_*` / `read_*` returns nothing / `Result<T>` | n/a | Idiomatic difference, not a divergence. Rust trades operator chaining for explicit error propagation via `?`. |
| `ByteBuffer::operator>>(bool&)` | `read_uint8()? > 0` (manual) / no `read_bool` | ⚠️ | C++ reads `char` and returns `> 0`; Rust callers do this by hand. Cosmetic. |
| `ByteBuffer::operator>>(float&)` | `read_float()` | ❌ behavioral divergence | C++ throws `ByteBufferInvalidValueException` if `!std::isfinite(value)` (ByteBuffer.cpp:47-53). **Rust accepts NaN/Inf silently** (world_packet.rs:238-241). A malicious or buggy client can send `0x7FC0_0000` (NaN) and downstream code may divide-by-NaN or misbehave. Sub-task #PACKETS.9. |
| `ByteBuffer::operator>>(double&)` | — | ❌ missing | No `read_double` / `read_f64` in Rust. (`grep f64 world_packet.rs` returns nothing.) Currently only u32-bits floats are used in shipped opcodes; double is rare in WoW protocol but exists (some movement extensions). |
| `ByteBuffer::ReadCString(requireValidUtf8=true)` | `read_cstring() -> Result<String>` | ✅ | Both walk to first 0 byte, both validate UTF-8 (Rust via `String::from_utf8`, errors → `PacketError::StringError`). Both gracefully handle missing terminator (return partial). Equivalent. |
| `ByteBuffer::ReadString(length, requireValidUtf8)` | `read_string(len) -> Result<String>` | ✅ | Equivalent. |
| `ByteBuffer::WriteString(str)` (no null) | — (only `write_string` writes raw bytes — works, but no length/no null) | ⚠️ | Rust `write_string` does the same as C++ `WriteString` (raw bytes, no null). C++ `operator<<(std::string)` is the null-terminating variant; Rust splits into `write_string` (no null) and `write_cstring` (with null). Naming is clearer than C++ but caller must remember. |
| `ByteBuffer::ReadPackedUInt64(uint64&)` | — (subsumed by `read_packed_guid`) | ⚠️ | No standalone 8-byte packed reader. `read_packed_guid` reads 16 bytes (low+high). If a future opcode needs the bare 64-bit form (e.g. for a non-GUID quantity packed by mask) it isn't available. |
| `ByteBuffer::AppendPackedUInt64(uint64)` | — | ⚠️ | Same as above on the write side. |
| `ObjectGuid` `operator<<`/`operator>>` (low+high mask, 128-bit) | `write_packed_guid(&ObjectGuid)` / `read_packed_guid()` | ✅ | Same algorithm: low_mask (1 byte), high_mask (1 byte), then non-zero bytes of low LE, then non-zero bytes of high LE. Verified against `src/server/game/Entities/Object/ObjectGuid.cpp:756-786`. wotlk_classic uses the 128-bit form (the §13 hypothesis "WotLK is 64-bit packed" was wrong for this branch). |
| `ByteBuffer::appendPackXYZ(x, y, z)` | `WorldPacket::write_packed_xyz` | ✅ | `MonsterMove` uses the packed-u32 helper for every `packed_deltas` entry. `monster_move_matches_real_cpp_waypoint_capture_bytes` verifies the compressed path byte-for-byte against a real C++ `SMSG_ON_MONSTER_MOVE` body. |
| `ByteBuffer::clear()` | — | ❌ missing | Reset write side without dropping the buffer. Rare. |
| `ByteBuffer::resize(n)` / `reserve(n)` | (`BytesMut` has its own) | ✅ | Different API surface; equivalent capability. |
| `ByteBuffer::DEFAULT_SIZE = 0x1000` | `BytesMut::with_capacity(64)` in `new_empty()` | ⚠️ | 64 vs 4096. For typical SMSG payloads (>64 bytes) the buffer grows several times. Minor perf impact, not correctness. Sub-task #PACKETS.11. |
| `MessageBuffer::Normalize` / ring semantics | — (no equivalent; `wow-network` uses fresh `BytesMut` per `read_exact`) | ⚠️ | The §11 note "Rust uses BytesMut directly with capacity pre-assigned" is partially correct — there is no ring/normalize. Each inbound packet is read into its own buffer (`world_socket.rs:513-547`). Functionally fine for TLS-framed messages; loses the alloc-amortization C++ gets from re-using one ring per session. Sub-task #PACKETS.12. |
| `ByteBufferException` family | `PacketError` enum | ✅ | Idiomatic Rust mapping. Variants: `ReadPastEnd`, `StringError`, `InvalidOpcode`, `TooLarge`, `DecompressionError`, `UnexpectedOpcode`. **Every read path verified to return `Err`, no `panic!`/`unwrap` in the read code path** (world_packet.rs:171-281). ✅ |

### 13.3 Critical findings

**A. NaN/Inf accepted on float read** — `read_float` (world_packet.rs:238-241) returns `Ok(f32::from_bits(u32))` unconditionally. C++ throws on `!std::isfinite` (ByteBuffer.cpp:47-53). Impact: a hostile client can poison movement/orientation with NaN that propagates through `glam::Vec3` math. Severity: medium. **#PACKETS.9.**

**B. `PutBits` / `bitwpos` missing** — required for the C++ pattern of writing a bit-packed count placeholder, then writing N entries, then back-patching the count. Without it, any port of such handler must use a two-pass collect-then-write idiom (already documented in `AGENTS.md` § "Patterns to follow"), which is fine for `Vec<Vec<u8>>` of *whole packets* but not for *bits inside one packet*. Severity: medium-high for any future ObjectMgr / movement-update port. **#PACKETS.8.**

**C. `appendPackXYZ` closed** — `WorldPacket::write_packed_xyz` mirrors
`ByteBuffer::appendPackXYZ` (ByteBuffer.h:583-590), and `MonsterMove` writes its
`packed_deltas` through that helper. `monster_move_matches_real_cpp_waypoint_capture_bytes`
now verifies the complete 117-byte body against a real C++ waypoint-movement capture.
**#PACKETS.10 / #NEXT.R8.ENTITIES.1237.**

**D. `wpos`/`bitwpos` setters missing** — the Rust API has no way to inspect or set the write cursor. Most handlers don't need this, but the family of "write placeholder, fill in later" patterns common in TC handlers cannot be ported directly. Severity: low until needed. Pairs with #PACKETS.8.

**E. `f64`/`double` read missing** — no `read_double` / `read_f64`. WoW protocol uses double rarely (some scenario / building data), so no immediate impact, but trivial to add. Severity: low. Add to #PACKETS.5 (extend `PacketError` / read fns).

**F. String encoding by opcode (sample of 5)** — verified hand-by-hand against C++ handler-side serialization. All five matched:

  - `auth.rs` realm-list realm names → `WriteBits(name.len, 8); WriteString(name)` (length-prefixed bits, no null). C++ `WorldSession::HandleRealmListOpcode` writes the same shape (TC `WoltkClassic/AuthChallenge.cpp`). ✅
  - `chat.rs` (CMSG_MESSAGECHAT_*): `read_bits(11)` for body length then `read_string(len)`. Matches C++ chat handlers. ✅
  - `character.rs` SMSG_ENUM_CHARACTERS_RESULT name field: `write_bits(name.len, 6); write_string(name)`. Matches `WorldSession::HandleCharEnum`. ✅
  - `query.rs` creature/quest names: `write_bits(len, 11); write_cstring(name)` — interesting case, **mixes both** length-prefix-via-bits AND null terminator. Matches C++ `CreatureTemplate::InitializeQueryData` which does `<< Name << uint8(0)` after a bit-packed length. ✅
  - `gossip.rs` option text: `write_bits(text.len, 12); write_string(text)`. Matches. ✅

  No string-encoding divergences in the sampled opcodes. The `read_string`/`read_cstring`/`write_string`/`write_cstring` four-fn surface is sufficient for all five.

**G. Bit-packing wire equivalence** — see §13.4. Confirmed identical bytes despite the inverted counter direction. The §8 "Suspicious / likely divergent" hypothesis was a false alarm.

### 13.4 Hand-traced bit-packing example

**Input:** `WriteBits(0b1011, 4); WriteBits(0b110, 3); FlushBits();`

**C++** (`ByteBuffer.h:175-207`, initial state `_bitpos=8, _curbitval=0`):

| Step | Call | `_bitpos` after | `_curbitval` after | Notes |
|---|---|---|---|---|
| init | — | 8 | 0x00 | `InitialBitPos = 8` |
| WriteBit(1) — MSB of 0b1011 | `--_bitpos; if(bit) _curbitval \|= 1<<_bitpos;` | 7 | 0x80 | bit→pos 7 |
| WriteBit(0) | — | 6 | 0x80 | no set |
| WriteBit(1) | `_curbitval \|= 1<<5` | 5 | 0xA0 | |
| WriteBit(1) | `_curbitval \|= 1<<4` | 4 | 0xB0 | end of `WriteBits(0b1011,4)` |
| WriteBit(1) — MSB of 0b110 | `_curbitval \|= 1<<3` | 3 | 0xB8 | |
| WriteBit(1) | `_curbitval \|= 1<<2` | 2 | 0xBC | |
| WriteBit(0) | — | 1 | 0xBC | end of `WriteBits(0b110,3)` |
| `FlushBits()` | `_bitpos=8; append(&_curbitval,1); _curbitval=0;` | 8 | 0x00 | **emits 0xBC** |

C++ wire output: `[0xBC]`.

**Rust** (`world_packet.rs:386-403`, initial state `wbit_pos=0, wbit_buf=0`):

| Step | Call | `wbit_pos` after | `wbit_buf` after | Notes |
|---|---|---|---|---|
| init | — | 0 | 0x00 | |
| write_bit(1) — MSB of 0b1011 | `wbit_pos+=1; wbit_buf \|= 1<<(8-wbit_pos);` → `1<<7` | 1 | 0x80 | |
| write_bit(0) | — | 2 | 0x80 | |
| write_bit(1) | `1<<5` | 3 | 0xA0 | |
| write_bit(1) | `1<<4` | 4 | 0xB0 | end of `write_bits(0b1011,4)` |
| write_bit(1) — MSB of 0b110 | `1<<3` | 5 | 0xB8 | |
| write_bit(1) | `1<<2` | 6 | 0xBC | |
| write_bit(0) | — | 7 | 0xBC | end of `write_bits(0b110,3)` |
| `flush_bits()` | `data.put_u8(wbit_buf); wbit_buf=0; wbit_pos=0;` | 0 | 0x00 | **emits 0xBC** |

Rust wire output: `[0xBC]`.

**Conclusion:** Both implementations produce the identical byte `0xBC` for this input. The two counter conventions converge on the same bit-position arithmetic:

```
C++   bit position written = _bitpos - 1   (after --_bitpos)   = 7, 6, 5, 4, 3, 2, 1, 0
Rust  bit position written = 8 - wbit_pos  (after ++wbit_pos)  = 7, 6, 5, 4, 3, 2, 1, 0
```

Identical. **`write_bits` and `read_bits` are wire-compatible with C++ TrinityCore wotlk_classic.**

### 13.5 Recommended sub-tasks

Append to §9 (priority H = wire-correctness, M = developer ergonomics, L = nice-to-have):

- [ ] **#PACKETS.8** Implement `put_bits(pos: usize, value: u32, n: u32)` + `bit_write_pos() -> usize` + `bit_write_pos_set(pos: usize)`. Required for back-patching bit-packed lengths. (M, blocks any movement-update / dynamic-list port that needs lazy length write) — H once that work starts.
- [ ] **#PACKETS.9** `read_float` / `read_double` reject NaN/Inf with `Err(PacketError::InvalidValue("float", "non-finite"))`. Add `InvalidValue { ty: &'static str, value: String }` variant to `PacketError`. (H — security/robustness)
- [x] **#PACKETS.10** `WorldPacket::write_packed_xyz(x, y, z)` implements ByteBuffer.h:583-590 (`((x/0.25) & 0x7FF) | ((y/0.25) & 0x7FF)<<11 | ((z/0.25) & 0x3FF)<<22` written as u32), and `MonsterMove` uses it for the compressed spline deltas. Exact C++ capture coverage: `#NEXT.R8.ENTITIES.1237`.
- [ ] **#PACKETS.11** Bump `BytesMut::with_capacity` in `new_empty()` from 64 → 1024 (or 4096 to match C++ DEFAULT_SIZE). Measure with criterion before/after on character-enum response. (L — perf only)
- [ ] **#PACKETS.12** Decide whether to add a ring-buffer / `Normalize`-like wrapper around `wow-network`'s read path, or document explicitly that one-shot `read_exact` per message is the chosen design. (L — currently fine)
- [ ] **#PACKETS.13** Add `read_double() -> Result<f64>` and `write_double(v: f64)`. (L)
- [ ] **#PACKETS.14** Add `write_position(&self) -> usize` and `set_write_position(pos: usize)` for back-patch byte writes (e.g., size headers). (M, often paired with #PACKETS.8)
- [ ] **#PACKETS.15** Replace **§8 status `✅` claim about strings/bit-packing/PackedGuid** with the more honest "core ops match wire; auxiliary ops missing — see §13".

Existing #PACKETS.1 (cross-impl bit roundtrip test) — superseded by §13.4; close it or convert to a regression test in `crates/wow-packet/tests/`.
Existing #PACKETS.2 (`ReadPackedGuid` / `WritePackedGuid`) — done (write_packed_guid, read_packed_guid, see ObjectGuid 128-bit confirmation in §13.2).
Existing #PACKETS.3 (string encoding per opcode) — sampled 5/many; close-to-done but the full sweep across all ~50 packet types remains future work.
Existing #PACKETS.4 (`append_bytes` / `read_bytes`) — done (`write_bytes`, `read_bytes`).
