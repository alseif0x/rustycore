# Migration: shared/Secrets

> Historical reference / dated audit, not current instructions or status.
> Use [STATE.md](STATE.md), [PORT_PLAN.md](PORT_PLAN.md) and the active issue/checkpoint.
> Technical anchors and findings below need re-contrast before use; old workflow,
> task order, percentages and validation gates do not govern new work.

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/server/shared/Secrets/`
> **Rust target crate(s):** `crates/wow-crypto/` (TODO) + `crates/bnet-server/` (TODO)
> **Layer:** L1
> **Status:** ❌ not started (~0%)
> **Audited vs C++:** ❌ confirmed not started — audit done 2026-05-01
> **Last updated:** 2026-05-01

---

## 1. Purpose

Manager de secretos persistentes del servidor (singleton `SecretMgr`) que carga claves desde el `.conf`, las verifica contra un digest Argon2 guardado en `secret_digest` (DB `auth`), y orquesta transiciones cuando el operador rota la clave (re-cifra todos los TOTP secrets de cuentas usando la clave nueva). El único secreto vivo en WoLK 3.4.3 es `SECRET_TOTP_MASTER_KEY`: la master key con la que se AES-cifran los secretos TOTP por cuenta.

---

## 2. C++ canonical files

<!-- REFINE.020:BEGIN canonical-file-coverage -->

### R2 canonical file coverage (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md`; C++ canonico: `/home/server/woltk-trinity-legacy/src/server/`. No valida que Rust este correcto.

| C++ file | Lines | Assignment basis |
|---|---:|---|
| `shared/Secrets/SecretMgr.cpp` | 237 | `prefix` |
| `shared/Secrets/SecretMgr.h` | 85 | `prefix` |

<!-- REFINE.020:END canonical-file-coverage -->

| File | Lines | Purpose |
|---|---|---|
| `src/server/shared/Secrets/SecretMgr.h` | 85 | Class declaration + `Secret` POD + `SecretOwner`/`Secrets` enums |
| `src/server/shared/Secrets/SecretMgr.cpp` | 237 | `Initialize`, `GetSecret`, `AttemptLoad`, `AttemptTransition` (re-cifra cuentas) |
| **TOTAL** | **~322** | — |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `SecretMgr` | singleton class | Holder + lifecycle de secretos del proceso |
| `SecretMgr::Secret` | inner struct | Estado lazy: `NOT_LOADED_YET / LOAD_FAILED / NOT_PRESENT / PRESENT`, mutex propio, `BigNumber value` |
| `Secrets` | enum | Índices: `SECRET_TOTP_MASTER_KEY = 0`, `NUM_SECRETS = 1` |
| `SecretOwner` | enum | `SECRET_OWNER_BNETSERVER` / `SECRET_OWNER_WORLDSERVER` (qué proceso es el dueño autoritativo) |
| `SecretInfo` (anon) | struct | Tabla estática: `configKey`, `oldKey`, `bits`, `owner`, `flags()` |
| `SecretFlags` | enum | `SECRET_FLAG_DEFER_LOAD = 0x1` (per-owner via macro `SECRET_FLAG_FOR`) |

Tabla `secret_info[]` contiene un solo entry: `{ "TOTPMasterSecret", "TOTPOldMasterSecret", 128, BNETSERVER, WORLDSERVER_DEFER_LOAD }` → en bnetserver carga eager, en worldserver carga lazy on first `GetSecret`.

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `SecretMgr::instance()` | Singleton accessor | — |
| `SecretMgr::Initialize(owner)` | Set `OWNER` global, cargar todos los `!DEFER_LOAD` secrets, abortar si fallan | `AttemptLoad` |
| `SecretMgr::GetSecret(i)` | Lazy-load on first call si `NOT_LOADED_YET`, retornar `Secret const&` | `AttemptLoad` |
| `SecretMgr::AttemptLoad(i, errorLevel, lock)` | Lee `LOGIN_SEL_SECRET_DIGEST`, lee config, verifica con Argon2, dispara transición si difieren | `Argon2::Verify`, `GetHexFromConfig`, `AttemptTransition` |
| `SecretMgr::AttemptTransition(i, newSecret, oldSecret, hadOldSecret)` | Decrypt todos los `account.totp_secret` con old key, re-encrypt con new key, regenerar digest Argon2, todo en una transacción | `LOGIN_UPD_ACCOUNT_TOTP_SECRET`, `LOGIN_DEL_SECRET_DIGEST`, `LOGIN_INS_SECRET_DIGEST`, `Argon2::Hash` |
| `Secret::operator bool()` | `state == PRESENT` | — |
| `Secret::operator*()` / `operator->()` | Acceso a `BigNumber value` | — |
| `Secret::IsAvailable()` | `state ∉ {NOT_LOADED_YET, LOAD_FAILED}` | — |
| `GetHexFromConfig(key, bits)` (free fn) | Lee config como hex, valida rango `[0, 2^bits)`, trunca con WARN si excede | `sConfigMgr->GetStringDefault`, `BigNumber::SetHexStr` |

---

## 5. Module dependencies

**Depends on:**
- `shared/Crypto/AES` — `AEEncryptWithRandomIV<AES>`, `AEDecrypt<AES>` (16-byte key)
- `shared/Crypto/Argon2` — `Argon2::Hash(secret, salt)`, `Argon2::Verify(secret, hash)`
- `shared/Crypto/CryptoGenerics` — wrappers genéricos
- `shared/Crypto/BigNumber` — almacena el secreto como `BigNumber` (≤128 bits)
- `shared/Database` — prep statements `LOGIN_SEL_SECRET_DIGEST`, `LOGIN_INS_SECRET_DIGEST`, `LOGIN_DEL_SECRET_DIGEST`, `LOGIN_UPD_ACCOUNT_TOTP_SECRET`, transactions
- `shared/Configuration` — `sConfigMgr->GetStringDefault`
- `shared/Logging` — TC_LOG (FATAL, ERROR, INFO)

**Depended on by:**
- `worldserver/Authentication/AuthenticationPackets` — al loggear, decifra `account.totp_secret` con `sSecretMgr->GetSecret(SECRET_TOTP_MASTER_KEY)` para validar TOTP code
- `bnetserver/RealmList` (indirecto via auth) — owner del secreto, lo carga eager

---

## 6. SQL / DB queries

| Statement | Purpose | DB |
|---|---|---|
| `LOGIN_SEL_SECRET_DIGEST` | `SELECT digest FROM secret_digest WHERE id = ?` | auth |
| `LOGIN_INS_SECRET_DIGEST` | `INSERT INTO secret_digest (id, digest) VALUES (?,?)` | auth |
| `LOGIN_DEL_SECRET_DIGEST` | `DELETE FROM secret_digest WHERE id = ?` | auth |
| inline `SELECT id, totp_secret FROM account` | Iterar todas las cuentas para re-cifrar TOTP en transición | auth |
| `LOGIN_UPD_ACCOUNT_TOTP_SECRET` | `UPDATE account SET totp_secret = ? WHERE id = ?` | auth |

---

## 7. Wire-protocol packets

N/A — módulo enteramente offline/setup.

---

## 8. Current state in RustyCore

<!-- REFINE.021:BEGIN rust-target-coverage -->

### R2 Rust target coverage (generated)

> Fuente: cabecera `Rust target crate(s)` y seccion 8 del doc; verificado contra `/home/server/rustycore`. Esto solo valida existencia/estado del target Rust, no correccion funcional contra C++.

| Rust target | Kind | Rust files | Lines | Status | Notes |
|---|---|---:|---:|---|---|
| `crates/wow-crypto` | `crate_dir` | 9 | 2327 | `exists_active` | crate exists |
| `crates/bnet-server` | `crate_dir` | 13 | 2831 | `exists_active` | crate exists |
| `crates/wow-database/src/statements/login.rs` | `file` | 1 | 327 | `exists_active` | file exists |
| `crates/world-server` | `crate_dir` | 1 | 818 | `exists_active` | crate exists |

<!-- REFINE.021:END rust-target-coverage -->

**Files in `/home/server/rustycore`:**
- **(ninguno)** — sin equivalente Rust de `SecretMgr`.
- `crates/wow-database/src/statements/login.rs` ya **declara** los prep statements: `SEL_SECRET_DIGEST`, `INS_SECRET_DIGEST`, `DEL_SECRET_DIGEST`, `SEL_ACCOUNT_TOTP_SECRET`, `UPD_ACCOUNT_TOTP_SECRET` — pero ninguno se invoca todavía.

**What's implemented:**
- Solo los SQL statements registrados (no usados).

**What's missing vs C++:**
- **Toda la lógica.** No hay struct `SecretMgr`, no hay `Secret`, no hay carga, no hay verificación Argon2, no hay transición.
- Sin `Argon2` wrapper en `wow-crypto` (verificar dependencia `argon2` crate; no aparece en grep).
- Sin AES key-wrap genérico para los TOTP secrets (`wow-crypto` tiene AES-GCM para network, no este modo).
- TOTP no implementado en `bnet-server` ni `world-server` — el handler de auth/login no consulta la master key.
- No existe `BigNumber` Rust (el SRP6 en `wow-crypto` usa `num-bigint` internamente, OK).

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- Como TOTP no está activo, **no se ha confirmado** que la decryption de `account.totp_secret` produce los mismos bytes que TrinityCore. Si se activa esto es el primer test de regresión obligatorio.
- 128 bits = AES-128 key → ojo con AES-256 si alguien “mejora” la spec (rompería compatibilidad).

**Tests existing:**
- 0.

---

## 9. Migration sub-tasks

<!-- REFINE.022:BEGIN task-wbs -->

### R2 Task WBS (generated)

> Fuente: `docs/migration/inventory/cpp-files-by-module.md` + targets verificados en `docs/migration/inventory/r2-rust-targets.tsv`. C++ sigue siendo el oraculo; estas tareas son el suelo de cobertura por archivo, no una prueba de port correcto.

- [ ] **#SHARED_SECRETS.WBS.001** Cerrar la migracion auditada de `shared/Secrets/SecretMgr.cpp`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/shared/Secrets/SecretMgr.cpp`
  Rust target: `crates/wow-crypto`, `crates/bnet-server`, `crates/world-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.
- [ ] **#SHARED_SECRETS.WBS.002** Cerrar la migracion auditada de `shared/Secrets/SecretMgr.h`
  C++ refs: `/home/server/woltk-trinity-legacy/src/server/shared/Secrets/SecretMgr.h`
  Rust target: `crates/wow-crypto`, `crates/bnet-server`, `crates/world-server`
  Depends on: #REFINE.020, #REFINE.021; execution order finalized by #REFINE.040
  Acceptance: Rust target compiles; behavior and public contracts are checked against the listed C++ file; unit/golden/integration tests are added or marked n/a with reason; divergences are recorded before closing.
  Notes: `ready_for_small_task`; Single source-file coverage task; split further if C++ review exposes multiple independent behaviors. Assignment basis: prefix.

<!-- REFINE.022:END task-wbs -->

- [ ] **#SEC.1** Añadir crate dep `argon2` (RustCrypto) en `wow-crypto/Cargo.toml`. (L)
- [ ] **#SEC.2** Implementar `wow_crypto::argon2::{hash, verify}` con parámetros TC-compatibles (Argon2id, mismas iters/memory que el C++ default — verificar `Argon2.cpp` upstream). (M)
- [ ] **#SEC.3** Implementar `wow_crypto::aes_keywrap::{encrypt_with_random_iv, decrypt}` para `Vec<u8>` payload con clave de 16 bytes (formato C++: `[IV(12)][cipher][tag(16)]` — confirmar). (M)
- [ ] **#SEC.4** Crear `crates/wow-crypto/src/secret_mgr.rs` con `SecretMgr`, `Secret`, enum `Secrets`, enum `SecretOwner`. (M)
- [ ] **#SEC.5** Implementar `SecretMgr::initialize(owner, config, login_db)` con eager-load para non-defer + abort on failure. (M)
- [ ] **#SEC.6** Implementar `get_secret(i) -> Result<&Secret>` con lazy-load + `parking_lot::Mutex` per-secret. (L)
- [ ] **#SEC.7** Implementar `attempt_load` con verificación Argon2 + dispatch a `attempt_transition`. (M)
- [ ] **#SEC.8** Implementar `attempt_transition` para `SECRET_TOTP_MASTER_KEY`: SELECT all accounts.totp_secret, decrypt-with-old, encrypt-with-new, UPDATE en una transacción. (H)
- [ ] **#SEC.9** Wire `SecretMgr::initialize` en `bnet-server/src/main.rs` (owner = BNETSERVER, eager). (L)
- [ ] **#SEC.10** Wire `SecretMgr::initialize` en `world-server/src/main.rs` (owner = WORLDSERVER, defer). (L)
- [ ] **#SEC.11** Implementar TOTP validation usando `get_secret(SECRET_TOTP_MASTER_KEY)` + `account.totp_secret` decrypt → RFC6238 (necesita crate `totp-rs` o equivalente). (M)
- [ ] **#SEC.12** Tests: round-trip encrypt/decrypt con master key, Argon2 verify positive/negative, transition de mock-DB. (M)

---

## 10. Regression tests to write

<!-- REFINE.024:BEGIN tests-required -->

### R2 Tests required (generated)

> Fuente: cobertura C++ asignada y targets Rust verificados. Estos gates son obligatorios para cerrar tareas WBS; `n/a` solo vale con razon explicita y referencia C++/producto.

| Gate | Required coverage | Acceptance |
|---|---|---|
| `#SHARED_SECRETS.TEST.001 / unit` | Unit tests for pure data structures, parsers, state transitions and edge cases directly ported from C++ invariants. C++ scope: 2 files / 322 lines; refs: `/home/server/woltk-trinity-legacy/src/server/shared/Secrets/SecretMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/shared/Secrets/SecretMgr.h`. Rust target: `crates/bnet-server`, `crates/world-server`, `crates/wow-crypto`. | `cargo test -p bnet-server && cargo test -p world-server && cargo test -p wow-crypto` passes for the touched target(s); every migrated behavior has focused tests or an explicit documented n/a. |
| `#SHARED_SECRETS.TEST.002 / golden` | Golden/fixture tests derived from C++ packet bytes, SQL rows, config defaults, DB2 records or deterministic algorithm outputs. C++ scope: 2 files / 322 lines; refs: `/home/server/woltk-trinity-legacy/src/server/shared/Secrets/SecretMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/shared/Secrets/SecretMgr.h`. Rust target: `crates/bnet-server`, `crates/world-server`, `crates/wow-crypto`. | Golden fixtures are checked in or generated by a documented harness; Rust output matches C++ semantics byte-for-byte where wire/data format is involved. |
| `#SHARED_SECRETS.TEST.003 / integration` | Integration tests for startup/load paths, database access, registry wiring and cross-crate behavior. C++ scope: 2 files / 322 lines; refs: `/home/server/woltk-trinity-legacy/src/server/shared/Secrets/SecretMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/shared/Secrets/SecretMgr.h`. Rust target: `crates/bnet-server`, `crates/world-server`, `crates/wow-crypto`. | The module can be loaded/exercised through its real Rust service boundary without panics, missing handlers or silent default-success paths. |
| `#SHARED_SECRETS.TEST.004 / e2e` | Client/bot or scripted runtime scenario when the module affects login, world session, packets, entities, maps, gameplay or content. C++ scope: 2 files / 322 lines; refs: `/home/server/woltk-trinity-legacy/src/server/shared/Secrets/SecretMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/shared/Secrets/SecretMgr.h`. Rust target: `crates/bnet-server`, `crates/world-server`, `crates/wow-crypto`. | Bot/client scenario covers the user-visible path, or the doc marks E2E n/a with a concrete product-scope reason before implementation closes. |

<!-- REFINE.024:END tests-required -->

- [ ] `Argon2::verify(secret, hash)` true para `(secret, Argon2::hash(secret, salt))` — round-trip
- [ ] `AEEncryptWithRandomIV` + `AEDecrypt` round-trip preserva bytes
- [ ] Decrypt de un `account.totp_secret` cifrado por el C++ TrinityCore con la misma master key produce los mismos bytes en Rust (test fixture binario)
- [ ] `attempt_load` aborta con FATAL si owner=BNETSERVER y digest no matchea
- [ ] `attempt_load` retorna `LOAD_FAILED` (sin abort) si owner=WORLDSERVER y digest no matchea
- [ ] `attempt_transition` re-cifra N cuentas y deja el digest nuevo; rollback en error
- [ ] Lazy load: primer `GetSecret` carga, segundos hits no re-cargan
- [ ] Concurrencia: dos hilos llamando `GetSecret` simultáneo solo cargan una vez (mutex per-secret)

---

## 11. Notes / gotchas

<!-- REFINE.025:BEGIN product-scope -->

### R2 Product scope / exclusions (generated)

> Fuente: cabecera del doc + inventario C++ asignado. Ninguna marca de alcance elimina C++ del backlog: solo define si se implementa, se sustituye por idiom Rust o se desactiva explicitamente para producto.

| Scope | Decision | C++ retained | Evidence |
|---|---|---|---|
| `active_port_scope` | Full C++ surface remains in migration scope; no product exclusion recorded. | 2 files / 322 lines; refs: `/home/server/woltk-trinity-legacy/src/server/shared/Secrets/SecretMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/shared/Secrets/SecretMgr.h` | `crates/wow-crypto/` (TODO) + `crates/bnet-server/` (TODO) \| ❌ not started (~0%) |

<!-- REFINE.025:END product-scope -->

<!-- REFINE.023:BEGIN known-divergences -->

### R2 Known divergences / bugs (generated)

> Fuente: C++ asignado en `cpp-files-by-module.md` + target Rust verificado en `r2-rust-targets.tsv`. Esto enumera divergencias estructurales conocidas; no sustituye la auditoria funcional contra C++ antes de cerrar tareas.

| ID | Rust evidence | C++ evidence | Status | Notes |
|---|---|---|---|---|
| `#SHARED_SECRETS.DIV.001` | _none generated_ | 2 C++ files / 322 lines assigned; refs: `/home/server/woltk-trinity-legacy/src/server/shared/Secrets/SecretMgr.cpp`, `/home/server/woltk-trinity-legacy/src/server/shared/Secrets/SecretMgr.h` | `no_generated_divergence` | No structural divergence found by target-existence scan; this is not a functional audit. |

<!-- REFINE.023:END known-divergences -->

1. **`SECRET_FLAG_DEFER_LOAD` es per-owner:** El macro `SECRET_FLAG(DEFER_LOAD, 0x1)` genera `BNETSERVER_DEFER_LOAD = 0x1 << 0` y `WORLDSERVER_DEFER_LOAD = 0x1 << 16`. La columna `_flags` empaqueta dos flags de 16 bits según owner. En la entry actual: `WORLDSERVER_DEFER_LOAD` significa “worldserver carga lazy”, `bnetserver` carga eager.
2. **AES key-wrap format:** `AEEncryptWithRandomIV<AES>` produce `[random IV][ciphertext][HMAC tag]`. NO es AES-GCM standard — TrinityCore usa AES-CBC + HMAC-SHA256 separado. **Crítico para compat de DB**: si Rust usa AES-GCM aquí los TOTP secrets existentes son ilegibles.
3. **`BigNumber` en C++ se serializa por `AsHexStr()`** para Argon2 input. En Rust replicar exactamente — strings hex sin prefijo, sin padding, lowercase (verificar).
4. **`ABORT()` semántica:** owner=true + load_failed = process abort. Es deliberado para que el operador no levante un servidor con TOTP roto silently.
5. **Missing `oldKey`** semántica: si `secret_info[i].oldKey == nullptr`, no hay path de transición — error duro.
6. **Salt 128 bits**: `salt.SetRand(128)`. Argon2 acepta salt arbitrario; usar `[u8; 16]`.
7. **TOTP no está en RustyCore todavía** — este módulo solo importa cuando se quiera habilitar 2FA. Bajar prioridad si TOTP no está en roadmap inmediato.

---

## 12. C++ → Rust mapping

| C++ | Rust | Notas |
|---|---|---|
| `class SecretMgr` (singleton) | `struct SecretMgr` + `OnceLock<SecretMgr>` o `Arc<SecretMgr>` en `AppState` | Singleton vía `OnceLock` |
| `SecretMgr::Secret` | `struct Secret { state: SecretState, value: Option<Vec<u8>> }` | `BigNumber` → `Vec<u8>` o `[u8; 16]` |
| `enum Secrets` | `enum Secret` (Rust enum sin payload) | discriminant |
| `enum SecretOwner` | `enum SecretOwner { Bnetserver, Worldserver }` | — |
| `std::mutex lock` per-Secret | `parking_lot::Mutex<SecretState>` | — |
| `BigNumber` | `[u8; 16]` (128 bits) o `num_bigint::BigUint` | Para AES key, prefer `[u8; 16]` |
| `Argon2::Hash` / `Verify` | crate `argon2` (RustCrypto) | Mismos params (Argon2id, m, t, p) que C++ |
| `AEEncryptWithRandomIV<AES>` | nuevo `wow_crypto::aes_cbc_hmac::seal` | **NO usar AES-GCM** — formato no compatible |
| `LoginDatabaseTransaction` | `sqlx::Transaction<'_, MySql>` | — |
| `sConfigMgr->GetStringDefault` | `wow_config::Config::get_string_default` | — |
| `ABORT()` | `panic!()` o `std::process::exit(1)` | Mismo efecto |
| Lazy load on `GetSecret` | `OnceCell` o check-and-set bajo Mutex | — |

---

*Template version: 1.0 (2026-05-01).*

---

## 13. Audit (2026-05-01)

### Findings table

| # | Pre-audit claim (sections 8–11) | Verified result | Evidence |
|---|---|---|---|
| 1 | `SecretMgr` 0% implemented — no struct, no Secret, no carga, no transición | **CONFIRMED.** `grep -ril -E "SecretMgr\|secret_mgr\|TOTPMaster\|TOTP_MASTER\|totp_master"` over `crates/` and `bins/` returns **zero matches**. No file references the symbol. | full-tree grep (negative) |
| 2 | SQL prep statements `SEL/INS/DEL_SECRET_DIGEST` + `SEL/UPD_ACCOUNT_TOTP_SECRET` declared but no callers | **CONFIRMED.** The only references in the workspace are the declarations themselves at `crates/wow-database/src/statements/login.rs:86-90` (variants) and `:248-252` (SQL strings). No `prepare(LoginStatements::*_SECRET_*)` or `prepare(LoginStatements::*_TOTP_SECRET)` exists anywhere in the codebase. | grep |
| 3 | TC encrypts TOTP with **AES-CBC + HMAC-SHA256** (per the doc body, sec. 11.2 + sec. 12 mapping note "NO usar AES-GCM") | **REFUTED.** TC actually uses **AES-128-GCM** with a 12-byte IV and a **truncated 12-byte tag**. See `src/common/Cryptography/AES.h:30-36` (`IV_SIZE_BYTES=12`, `KEY_SIZE_BYTES=16`, `TAG_SIZE_BYTES=12`) and `AES.cpp:25` (`EVP_aes_128_gcm()`). The `AEEncryptWithRandomIV` template (`CryptoGenerics.h:62-79`) appends `[ciphertext][iv(12)][tag(12)]` — *not* a separate HMAC. The doc's repeated claim "NO usar AES-GCM" is wrong; the right pattern is "use AES-128-GCM with a non-standard 12-byte tag". | C++ source |
| 4 | No Argon2 wrapper in Rust | **CONFIRMED.** `wow-crypto/Cargo.toml` does not depend on the `argon2` crate (only `aes-gcm`, `aes`, `hmac`, `sha2`, `sha1`, `pbkdf2`, `ed25519-dalek`, `rsa`). No `Argon2` symbol exists in `crates/wow-crypto/src/`. | Cargo.toml + grep |
| 5 | Master `TOTPMasterSecret` config key not read | **CONFIRMED.** Full-tree grep for `TOTPMasterSecret\|TOTPOldMasterSecret` returns only the doc itself. `wow-config` has no key for it. | grep |
| 6 | wow-crypto has no AE keywrap for the TOTP secret payload | **CONFIRMED.** `wow_crypto::world_crypt` uses `AesGcm<Aes128, U12, U12>` but is scoped to per-connection world-packet stream crypto (encrypt/decrypt opcodes). It is **not** generalised into an `AEEncryptWithRandomIV<Cipher>` analogue and is not callable on arbitrary Vec<u8> payloads. A new keywrap module is still required. | `wow-crypto/src/world_crypt.rs:24-36` |

### Critical findings

1. **Doc body is wrong about TC's AES mode.** Lines 105-106 (sec. 8 "What's missing") and especially lines 152, 172 ("**NO usar AES-GCM** — formato no compatible") direct the migration toward AES-CBC + HMAC. The actual TC scheme is AES-128-GCM with a 12-byte tag. Existing `wow_crypto::world_crypt::WowAesGcm = AesGcm<Aes128, U12, U12>` already has the right primitive — it just needs to be re-used with a different IV layout (TC suffixes IV+tag to ciphertext; world_crypt generates IV from a counter). Doc must be corrected before any implementation work begins, otherwise existing TC databases will be unreadable.
   - C++ refs: `src/common/Cryptography/AES.h:30-36`, `AES.cpp:25`, `CryptoGenerics.h:62-79`, `Secrets/SecretMgr.cpp:194,200` (call sites).
   - Rust ref: `crates/wow-crypto/src/world_crypt.rs:24,36`.
2. **No callers of the secret-related SQL.** The five prepared statements in `crates/wow-database/src/statements/login.rs:86-90` and `:248-252` are dead code at the call-graph level. There's no risk to leaving them, but the inventory ✅ for "SQL registered" was misread as ✅ for "module operational".
3. **TOTP login path silently passes 2FA-protected accounts.** Because `bnet-server` never reads `account.totp_secret` and never validates a TOTP code, any account that has a non-NULL `totp_secret` in the DB can be logged into with password alone. Docs should explicitly call this out as a security regression vs C++ TC.

### Status verdict

**Keep ❌ (~0%) — but lock the doc before any work starts.** The status was already ❌; no downgrade needed. The audit's *real* product is correcting the AES-CBC / AES-GCM confusion that would have cost a sprint of wasted work.

### Recommended sub-task priority shuffle

| Old order | New order | Reason |
|---|---|---|
| #SEC.3 (AES keywrap) framed as **"AES-CBC + HMAC"** | **rewrite** as "AES-128-GCM with `[ct][iv(12)][tag(12)]` framing — refactor existing `world_crypt::WowAesGcm` into a generic `wow_crypto::ae_keywrap::{seal, open}` taking `&[u8; 16]` key, IV from `OsRng`, returning `Vec<u8>` with the trailing tag layout TC uses." Complexity drops from **M** → **L** because the AEAD primitive already exists. | Sec.11.2 + sec.12 mapping rows are wrong. |
| #SEC.1 (`argon2` crate dep) | **same** L | Still required. |
| #SEC.2 (Argon2 wrapper) | **same** M, but pin params: `Argon2id`, `t=10`, `m=2^17 KiB`, `p=1`, `outputLen=16`. Source: `Argon2.h:30-34`. | Doc didn't pin the constants; the Argon2 crate's defaults are different. |
| #SEC.11 (TOTP RFC6238) | **deprioritise** until #SEC.4-#SEC.10 are landed; TOTP itself is ~2h, the encryption envelope is the hard part. | Smaller blast radius if shipped late. |
| **(new) #SEC.0** | "Correct the doc body — replace every 'AES-CBC + HMAC' / 'NO usar AES-GCM' with 'AES-128-GCM, 12-byte tag'." Complexity **L**. Must precede any code. | Avoid mis-implementation. |

### Header status

Updated to **❌ confirmed not started** (the audit confirmed the existing ❌, no upgrade or downgrade applies). The "Audited vs C++" line is now `❌ confirmed not started — audit done 2026-05-01` so future readers know it was checked rather than skipped.

