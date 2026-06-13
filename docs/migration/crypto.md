# Migration: Crypto

> **C++ canonical path:** `/home/server/woltk-trinity-legacy/src/common/Cryptography/`
> **Rust target crate(s):** `crates/wow-crypto/`
> **Layer:** L1
> **Status:** ✅ done (~95%)
> **Audited vs C++:** ⚠️ (see §13)
> **Last updated:** 2026-05-01

---

## 1. Purpose

Implementa criptografía de TrinityCore: SRP6 (auth), AES-128-GCM (packet encryption con tag de 12 bytes — variante WoW, no estándar), HMAC-SHA1/SHA256, RSA, Ed25519 y derivación de claves vía `SessionKeyGenerator`. WoLK 3.4.3 específicamente usa SRP6-Grunt + AES-128-GCM + HMAC-SHA256 para session keys. Rust traduce todo a `wow-crypto` con dependencia de OpenSSL/RustCrypto crates.

---

## 2. C++ canonical files

| File | Lines | Purpose |
|---|---|---|
| `src/common/Cryptography/Authentication/SRP6.h` | 242 | SRP6 base + GruntSRP6 (WoLK) + BnetSRP6 v1/v2 |
| `src/common/Cryptography/Authentication/SRP6.cpp` | ~800 | Verify, evidence, KDF |
| `src/common/Cryptography/AES.h` | 52 | AES-128-GCM wrapper (EVP_CIPHER_CTX) |
| `src/common/Cryptography/AES.cpp` | 87 | AES init, encrypt/decrypt, integrity |
| `src/common/Cryptography/Authentication/WorldPacketCrypt.h` | 44 | Packet-level encrypt state (ciphers + counters) |
| `src/common/Cryptography/Authentication/WorldPacketCrypt.cpp` | ~250 | Encrypt/decrypt con nonce per direction |
| `src/common/Cryptography/HMAC.h` | 140 | Template `HMAC<Hash>` (SHA1, SHA256) |
| `src/common/Cryptography/SessionKeyGenerator.h` | 61 | KDF de session key SRP a múltiples claves |
| `src/common/Cryptography/BigNumber.h` | ~470 | Wrapper OpenSSL BN_* |
| `src/common/Cryptography/BigNumber.cpp` | ~450 | Mul/mod/power/import/export |
| `src/common/Cryptography/CryptoHash.h` | 146 | SHA1/256/512 EVP wrappers |
| `src/common/Cryptography/CryptoRandom.h` | 46 | RAND_bytes init |
| `src/common/Cryptography/RSA.h` | 113 | RSA sign/verify (EVP_PKEY) |
| `src/common/Cryptography/Ed25519.h` | 69 | Ed25519 sign/verify |
| `src/common/Cryptography/ARC4.h` | 58 | RC4 stream (legacy, deprecated) |
| `src/common/Cryptography/Argon2.h` | 44 | Argon2 password hashing (BNet v2 opt) |
| `src/common/Cryptography/TOTP.h` | 37 | TOTP 2FA |
| `src/common/Cryptography/CryptoConstants.h` | 37 | Magic seeds y constantes |
| `src/common/Cryptography/OpenSSLCrypto.h` | 36 | OpenSSL init/cleanup |
| **TOTAL** | **~2933** | — |

---

## 3. Classes / Structs / Enums

| Symbol | Kind | Purpose |
|---|---|---|
| `SRP6` | abstract class | Interfaz SRP-6 base |
| `GruntSRP6` | class | WoLK SRP6 (N/g/k Blizzard, SHA-1, 256-bit) |
| `BnetSRP6Base` | class | BNet SRP6 base (v1 SHA1, v2 SHA256) |
| `BnetSRP6v1Base` / `BnetSRP6v2Base` | class | BNet variants (1 iter SHA-256 / 15k iter PBKDF/Argon2 SHA-512) |
| `WorldPacketCrypt` | class | AES + counters per direction |
| `AES` | class | AES-128-GCM EVP wrapper |
| `BigNumber` | class | Arbitrary precision int (BN_*) |
| `GenericHMAC<HashImpl>` | template class | HMAC any hash |
| `SessionKeyGenerator<Hash>` | template class | KDF para múltiples claves desde session key |
| `CryptoHash` (SHA1/256/512) | template class | Hashes EVP_MD wrappers |
| `RSA`, `Ed25519` | class | Signature algos |

---

## 4. Critical public methods / functions

| Symbol | Purpose | Calls into |
|---|---|---|
| `SRP6::VerifyClientEvidence(A, M1)` → `Optional<BigNumber>` | Valida CMSG_AUTH_SESSION proof | `DoVerifyClientEvidence` |
| `GruntSRP6::CalculateServerEvidence(A, M1, K)` → `BigNumber` | Genera M2 (server proof) | SHA1 interleave |
| `SRP6::CheckCredentials(user, pwd)` | Verifica vs salt/verifier stored | `CalculateX`, `CalculateVerifier` |
| `SRP6::MakeRegistrationData<Impl>(user, pwd)` | Genera (salt, verifier) registro | `CalculateVerifier` |
| `WorldPacketCrypt::Init(key)` | Init AES bidirec + counters | `AES::Init` |
| `WorldPacketCrypt::DecryptRecv(data, len, tag)` | Descifra cliente→server | `AES::Process`, counter++ |
| `WorldPacketCrypt::EncryptSend(data, len, tag)` | Cifra server→cliente | `AES::Process`, counter++ |
| `WorldPacketCrypt::PeekDecryptRecv(data, len)` | Descifra sin tag verify | `AES::ProcessNoIntegrityCheck` |
| `AES::Init(key)` / `Process(iv, data, len, tag)` | EVP setup + encrypt/decrypt | OpenSSL EVP_Cipher* |
| `BigNumber::ModPow(exp, mod)` → `BigNumber` | g^b mod N | OpenSSL BN_mod_exp |
| `BigNumber::operator*(const)` | Multiply | OpenSSL BN_mod_mul |
| `BigNumber::ToByteArray<N>(le)` | Export endian | memcpy |
| `GenericHMAC::GetDigestOf(seed, data)` → `Digest` | Compute HMAC | EVP_DigestSign |
| `SessionKeyGenerator::Generate(buf, sz)` | Fill buf con material | fill_up loop |

---

## 5. Module dependencies

**Depends on:**
- `BigNumber.h`, `CryptoHash.h`, `CryptoRandom.h`, `Define.h`, `Errors.h`
- **OpenSSL** (libssl, libcrypto): EVP_*, BN_*, RAND_*
- **Argon2** (opt, BNet v2 2FA)

**Depended on by:**
- `game/Server/WorldSocket` — `WorldPacketCrypt`, SRP6 verify
- `bnetserver/Authentication` — BnetSRP6 v1/v2
- `Account` service — session key storage en tabla `account.sessionkey`

---

## 6. SQL / DB queries

| Statement / Field | Purpose | DB |
|---|---|---|
| `account.salt` (32 bytes) | SRP6 salt | auth |
| `account.verifier` (256 bytes) | SRP6 verifier (hex) | auth |
| `account.sessionkey` (40 bytes hex) | Session key derivada (SHA-256) | auth |
| `SELECT verifier, salt FROM account WHERE id=?` | Pre-auth lookup | auth |
| `UPDATE account SET sessionkey=?, last_ip=? WHERE id=?` | Almacena session key post-auth | auth |

DBC/DB2: ninguno (Crypto es algorítmico puro).

---

## 7. Wire-protocol packets

Crypto NO origina packets; `WorldSocket` los emite. Crypto cifra:

| Opcode | Originador | Cifrado | Notas |
|---|---|---|---|
| `CMSG_AUTH_SESSION` | Client | NO yet | Handshake phase |
| `SMSG_AUTH_RESPONSE` | Server | NO yet | — |
| `SMSG_ENTER_ENCRYPTED_MODE` | Server | NO yet (Ed25519 signed) | Activa cifrado |
| `CMSG_ENTER_ENCRYPTED_MODE_ACK` | Client | NO yet | Ack |
| **All subsequent** | Both | **AES-128-GCM (12-byte tag)** | Cifrado |

---

## 8. Current state in RustyCore

**Files in `/home/server/rustycore`:**
- `crates/wow-crypto/src/srp6.rs` — SRP6 N/g/k WoLK, X/S/v calc, M1/M2 verify
- `crates/wow-crypto/src/bnet_srp6.rs` — BNetSRP6 v1/v2
- `crates/wow-crypto/src/world_crypt.rs` — AES-128-GCM con tag 12 bytes, counter per direction
- `crates/wow-crypto/src/session_key.rs` — SessionKeyGenerator SHA1/SHA256
- `crates/wow-crypto/src/hmac_utils.rs` — HMAC SHA1/SHA256
- `crates/wow-crypto/src/ed25519ctx.rs` — Ed25519 wrapper
- `crates/wow-crypto/src/sarc4.rs` — RC4 legacy
- `crates/wow-crypto/src/rsa_sign.rs` — RSA sign/verify

**What's implemented:**
- SRP6 completo (verify client evidence, compute server evidence, registration data)
- BNetSRP6 v1 + v2 (SHA-256, SHA-512 con iteraciones)
- AES-128-GCM con tag 12 bytes (NO el 16 estándar) — ✅ confirma
- Per-direction counter (server/client) en nonce
- SessionKeyGenerator split-half KDF
- HMAC-SHA1, HMAC-SHA256
- Ed25519 sign/verify

**What's missing vs C++:**
- **Vector tests** SRP6 — no hay tests vs salida C++ reference
- **AES nonce reuse detection** — sin auditoría de counter overflow approach
- **Argon2** — no implementado (opt para BNet v2 2FA, low priority)
- **TOTP** — no implementado (2FA, low priority)
- **ARC4** — implementado pero deprecated, no usar

**Suspicious / likely divergent (hipótesis pre-auditoría):**
- **Nonce format:** ¿`[counter LE | suffix LE]` vs C++ `[counter LE | suffix BE]`? **CRÍTICO VERIFICAR**.
- **HMAC key ordering:** orden estricto en derivación de DEC_KEY/ENC_KEY/HMAC keys; verificar.
- **SRP6 prime constants:** N exact hex `894B645E89E1535BBDAD5B8B290650530801B18EBFBF5E8FAB3C82872A3E9BB7`?
- **CalculateX:** `SHA1(salt || SHA1(user:pass))` — orden de concat exacto?
- **Ed25519 key:** `ENTER_ENCRYPTED_MODE_PRIVATE_KEY` (world_socket.rs:81) hardcoded; en producción real ¿viene de `build_info` table?

**Tests existing:**
- Algunos tests de round-trip pero sin vectors cross-impl
- `srp6.rs`: sin tests unitarios formales

---

## 9. Migration sub-tasks

- [ ] **#CRYPTO.1** Vector tests SRP6: dado `(user, pwd)` → comparar `(salt, verifier, A, M1, M2)` vs C++ reference output. (H)
- [ ] **#CRYPTO.2** Auditar nonce format: `[counter LE | suffix LE]` o BE — verificar contra C++ `WorldPacketCrypt::Encrypt`. (M)
- [ ] **#CRYPTO.3** Auditar HMAC-SHA256 key derivation order vs C++ exacto. (M)
- [ ] **#CRYPTO.4** Verificar SRP6 N, g, k = Blizzard wotlk_classic exactos. (L)
- [ ] **#CRYPTO.5** Test AES round-trip: encrypt header → decrypt → match plaintext. (M)
- [ ] **#CRYPTO.6** Counter overflow detection: warn si counter > 2^60. (L)
- [ ] **#CRYPTO.7** Test SessionKeyGenerator: seed → 32/40 bytes match C++ output. (M)
- [ ] **#CRYPTO.8** Documentar nonce construction (12 bytes) en world_crypt.rs comment. (L)
- [ ] **#CRYPTO.9** BNet SRP6: tests v1 SHA-256 iteration vs v2 PBKDF/Argon2. (M)
- [ ] **#CRYPTO.10** RSA signature verify: integrar pub key de `build_info` table, verificar EnterEncryptedMode. (H)
- [ ] **#CRYPTO.11** Ed25519 key: investigar si hardcoded vs DB-driven y unificar. (M)
- [ ] **#CRYPTO.12** Benchmark SRP6 verify vs C++. (M)
- [ ] **#CRYPTO.13** Argon2 PBKDF (opt para 2FA BNet v2). (M, low prio)
- [ ] **#CRYPTO.14** Fuzz testing: invalid SRP6 A values (0, N, N+1, neg mod). (H)
- [ ] **#CRYPTO.15** Documentar magic seeds (AUTH_CHECK_SEED, SESSION_KEY_SEED). (L)

---

## 10. Regression tests to write

- [ ] SRP6 verifier generation matches C++ bit-for-bit
- [ ] AES-GCM nonce counter increments monotonic
- [ ] 12-byte GCM tag (no 16) enforced
- [ ] Nonce per-direction (server vs client suffix)
- [ ] SessionKeyGenerator split-half output match C++
- [ ] HMAC-SHA256 key order (DEC, ENC, DEC_HMAC, ENC_HMAC)
- [ ] SRP6 evidence M1 rejection on invalid A (0 or N)
- [ ] Packet encryption round-trip
- [ ] Counter overflow warning approach 2^60

---

## 11. Notes / gotchas

1. **SRP6 safe primes:** N, g, k son específicos Blizzard WoLK. Hex N debe ser `894B645E89E1535BBDAD5B8B290650530801B18EBFBF5E8FAB3C82872A3E9BB7` exacto. Cualquier desviación → todos los logins rotos.
2. **HMAC key order STRICT:** SessionKeyGenerator splits seed mitades, hashea cada una, produce o1/o2. Order matters; replay detection depende de eso.
3. **AES-GCM 12-byte tag:** Standard GCM = 16. WoW = 12. Crate `aes-gcm` soporta `Tag<U12>`. ✅ Rust ya usa eso.
4. **Nonce reuse = RIP:** Counter overflow tras 2^64 packets. En práctica imposible (el server reiniciaría primero), pero merece warning en logs si se acerca a 2^60.
5. **Ed25519 key hardcoded en `world_socket.rs:81`:** En TrinityCore real esto viene de tabla `build_info` per build. Investigar si simplificación o regresión.
6. **ARC4 deprecated:** SRP6 Auth en WoLK NO usa RC4. Solo legacy code path; ignorar.
7. **BNet vs WoW:** `GruntSRP6` = WoLK world auth. `BnetSRP6 v1/v2` = Battle.net login. **Completamente separados** — no confundir.
8. **PBKDF vs Argon2:** BNet v2 usa iterations (no Argon2 en TrinityCore legacy). Argon2 es future-proof, opcional.

---

## 12. C++ → Rust mapping

| C++ | Rust | Notas |
|---|---|---|
| `GruntSRP6` (class) | `srp6::Srp6Params` + funciones (proc style) | — |
| `BigNumber` | `num_bigint::BigUint` | Direct replacement |
| `HMAC<SHA1/SHA256>` | `hmac_utils::HmacSha1/256` | Wrapper |
| `SessionKeyGenerator<SHA1/256>` | `SessionKeyGenerator{,256}` | Direct |
| `WorldPacketCrypt` | `WorldCrypt` (en `world_crypt.rs`) | aes-gcm crate, tag 12B |
| `AES::Process()` | `WorldCrypt::encrypt_packet()` / `decrypt_packet()` | Integrado |
| `uint64 _clientCounter` | `client_counter: u64` | — |
| `IV [12]` | `nonce: [u8; 12]` | counter+suffix |
| `Tag [12]` | `tag: [u8; 12]` | aes-gcm output |
| `CryptoHash SHA1` | `sha1::Sha1` (digest crate) | Extern |
| `CryptoRandom` | `rand::thread_rng()` | Extern |
| `RSA sign/verify` | `wow-network::rsa_sign` (separate module) | — |
| `Ed25519` | `ed25519_dalek` + `ed25519ctx` wrapper | — |

---

## 13. Audit (2026-05-01)

> Audited 2026-05-01 by sub-agent against C++ at `/home/server/woltk-trinity-legacy` commit `5100ce3d8fc6` ("WIP: consolidate pending changes before bot testing"). Scope: every primitive listed in this document plus the AuthSession HMAC flow and EnterEncryptedMode Ed25519ctx signing path. Authoritative C++ paths: `src/common/Cryptography/` and `src/server/game/Server/WorldSocket.cpp`.

> **Heads-up about the doc itself.** Some of the suspicions in §8 ("HMAC keying", "ServerToClient\0 / ClientToServer\0 KDF labels", "nonce format LE/BE") are based on a different protocol family. The legacy 3.4.3 wotlk_classic C++ in this tree does **not** use ASCII KDF labels; the per-direction binding is the 4-byte IV magic suffix `0x52565253` ("SRVR") / `0x544E4C43` ("CLNT") appended to the counter. There is no string `"ServerToClient"` or `"ClientToServer"` anywhere in the canonical C++. The Rust matches that. The audit below grades against the actual C++ behaviour, not the §8 hypotheses.

### 13.1 Primitive-by-primitive

| Primitive | C++ ref | Rust ref | Status | Divergence |
|---|---|---|---|---|
| SRP6 prime `N` (Grunt, 256-bit) | `src/common/Cryptography/Authentication/SRP6.cpp:27` | `crates/wow-crypto/src/srp6.rs:25` | ✅ | Both are `894B645E89E1535BBDAD5B8B290650530801B18EBFBF5E8FAB3C82872A3E9BB7` exact. |
| SRP6 generator `g`, multiplier `k` (Grunt) | `SRP6.cpp:28`, `SRP6.cpp:82` | `srp6.rs:31, 36` | ✅ | `g=7, k=3`. |
| BNet SRP6 v1 prime / `g` | `SRP6.cpp:30-31` | `bnet_srp6.rs:26-31, 46` | ✅ | 1024-bit prime + `g=2` byte-identical. |
| BNet SRP6 v2 prime / `g` | `SRP6.cpp:33-34` | `bnet_srp6.rs:34-43, 46` | ✅ | RFC 3526 group-14 + `g=2` byte-identical. |
| `CalculateX` (Grunt) hash input | `SRP6.cpp:91-97` `SHA1(salt, SHA1(user, ":", pwd))`; callers normalise via `Utf8ToUpperOnlyLatin` | `srp6.rs:107-117` + `wow_core::utf8_to_upper_only_latin_like_cpp` | ✅ | Rust normalises internally with the C++ Basic-Latin-only helper. This preserves current API ergonomics while matching the C++ account paths for valid UTF-8 input. |
| Verifier endianness on hash→bignum | C++ `SHA1::Digest → BigNumber(_, true)` is **LE** by default | `srp6.rs:116` `from_bytes_le` | ✅ | Match. |
| Wire endian for `A`, `B`, `salt` (Grunt) | `BigNumber::ToByteArray<32>()` default = LE (`SRP6.cpp:104, 110-115`) | `biguint_to_le_fixed(_, 32)` (`srp6.rs:165-168, 250-264`) | ✅ | Both LE 32-byte. |
| `M1` evidence formula (Grunt) | `SRP6.cpp:115` `H(NgHash, H(I), s, A, B, K)` | `srp6.rs:241-274` | ✅ | Formula and concatenation order identical, K is 40 bytes both sides. |
| `M2` evidence formula (Grunt) | `SRP6.cpp:88` `H(A, M1, K)` | `srp6.rs:281-288` | ✅ | Identical. |
| `SHA1Interleave` (split S, skip leading zeros) | `SRP6.cpp:122-150` | `srp6.rs:197-234` | ✅ | Algorithm matches incl. odd-byte skip. |
| BNet SRP6 `k = H(N || pad || g)` | `SRP6.h:196-219` (`ToByteArray<128/256>(false)`) | `bnet_srp6.rs:297-330` | ✅ | C++ pads both sides to N-size BE; Rust composes equivalent buffer (works because `g=2` fits in 1 byte). |
| BNet SRP6 evidence "broken padding" | `SRP6.cpp:183-187` `(bits + 8) >> 3` | `bnet_srp6.rs:419-432` | ✅ | Identical. |
| BNet SRP6 v2 `x = PBKDF2-HMAC-SHA512` 15000 iter | `SRP6.cpp:202-221` | `bnet_srp6.rs:342-376` | ✅ | Iterations, hash, sign-fix all match. |
| AES-128-GCM key/IV/tag sizes | `AES.h:30-36` `IV=12, KEY=16, TAG=12` | `world_crypt.rs:24, 36` | ✅ | **12-byte tag explicitly enforced** via `AesGcm<Aes128, U12, U12>`. NOT the 16-byte default. |
| GCM nonce layout | `WorldPacketCrypt.cpp:33-42` `[counter (memcpy uint64) | magic (memcpy uint32)]` on x86 = LE | `world_crypt.rs:100-105` `[counter LE 8 | suffix LE 4]` | ✅ | Byte-for-byte identical on little-endian hosts (the only platform either side runs on). |
| Direction magic constants | `WorldPacketCrypt.cpp:48, 60, 75` `0x544E4C43`(CLNT, recv), `0x52565253`(SRVR, send) | `world_crypt.rs:27-29` | ✅ | Identical. Test on `world_crypt.rs:299-305` asserts the LE bytes spell "CLNT"/"SRVR". |
| Per-direction monotonic counter | `WorldPacketCrypt.cpp:67, 82` | `world_crypt.rs:134, 170` | ✅ | Both `++` on every successful Process; never reused, never decremented. |
| Counter overflow handling | C++ no special handling (UB on `uint64` wrap) | Rust `+=` panics on debug, wraps on release | ⚠️ | Same practical risk as C++. No detection on either side. Sub-task `#CRYPTO.6` still open. |
| `SessionKeyGenerator<SHA256>` algorithm | `SessionKeyGenerator.h:23-58` `o0=H(o1‖o0‖o2); halves on init` | `session_key.rs:88-128` | ✅ | Constructor split, fill-up triple-hash, byte-by-byte refill loop all match. SHA1 variant matches too. |
| AuthSession digest HMAC | `WorldSocket.cpp:710-723` `H1=SHA256(KeyData‖Win64AuthSeed); HMAC-SHA256(H1, LocalChallenge‖ServerChallenge‖AuthCheckSeed)` | `world_socket.rs:296-326` | ✅ | Sequence and concatenation order identical, **including the asymmetry** (LocalChallenge first here). Rust compares first 24 bytes (`world_socket.rs:329`); C++ compares `Digest.size()` which is 24 (`AuthenticationPackets.h:80`). |
| Session-key HMAC | `WorldSocket.cpp:733-744` `HMAC-SHA256(SHA256(KeyData), ServerChallenge‖LocalChallenge‖SessionKeySeed) → SessionKeyGenerator<SHA256> → 40 bytes` | `world_socket.rs:348-367` | ✅ | Order **(server, local)** matches; this is intentionally reversed vs the digest step on both sides. |
| Encryption-key HMAC | `WorldSocket.cpp:746-753` `HMAC-SHA256(_sessionKey, LocalChallenge‖ServerChallenge‖EncryptionKeySeed)[..16]` | `world_socket.rs:369-379` | ✅ | Order **(local, server)**, truncate to 16 bytes — matches. |
| Magic seeds (`AuthCheckSeed`, `SessionKeySeed`, `EncryptionKeySeed`, `ContinuedSessionSeed`, `EnableEncryptionSeed`, `EnableEncryptionContext`) | `WorldSocket.cpp:55-58`, `AuthenticationPackets.cpp:347-348` | `world_socket.rs:46-75` | ✅ | All six 16-byte arrays byte-identical. |
| EnterEncryptedMode signing | `AuthenticationPackets.cpp:350-365` `Ed25519.SignWithContext(HMAC-SHA256(EncryptKey, [enabled]‖EnableEncryptionSeed), EnableEncryptionContext)` (= Ed25519ctx phflag=0) | `world_socket.rs:943-956` + `ed25519ctx.rs:38-88` | ⚠️ | Algorithm matches (same dom2 prefix, same context, same toSign). **But** the Ed25519 private key is hardcoded at `world_socket.rs:81-86`. C++ loads it from `EnterEncryptedModeSigner` (PEM file) per `build_info`. If the hardcoded Rust key does not match the public key the live client expects for this build, every login fails the signature check. Verify against the actual client build's pubkey in `BuildInfo` table. |
| HMAC-SHA1 / HMAC-SHA256 keying | `HMAC.h` (template `GenericHMAC`, `EVP_DigestSign`) | `hmac_utils.rs:14-19, 47-52` | ✅ | RFC 2202 / 4231 KAT vectors pass (`hmac_utils.rs:78-97`). |
| RC4 (legacy) | `ARC4.cpp` (deprecated, **never instantiated** in WorldSocket.cpp for this build) | `sarc4.rs` (dead code, no callers) | ✅ | Both sides agree it is unused for 3.4.3 wotlk_classic. |
| Argon2, TOTP | C++ has them (`Argon2.cpp`, `TOTP.cpp`) | Not implemented in Rust | ❌ | Not on the wire path for WoLK 3.4.3 world auth; only relevant if BNet v2 2FA is exposed. Acceptable gap, low priority. |
| Test coverage: round-trip with **real** captured vectors | C++ has unit tests in TC mainline | `wow-crypto/src/*.rs` 46 tests, but **zero** hardcoded vectors derived from a C++ run or wire capture | ⚠️ | Self-tests only verify internal consistency, not byte-equivalence with C++ output. Any silently-divergent byte order would not be caught by the existing tests except indirectly via on-the-wire login. |

### 13.2 Critical findings

1. **⚠️ Ed25519 private key is hardcoded** — `crates/wow-network/src/world_socket.rs:81-86`. C++ loads it from `EnterEncryptedModeSigner` configured from a PEM file, with one signer per client build (`AuthenticationPackets.cpp:359` uses `*EnterEncryptedModeSigner`). If the embedded Rust seed does not produce the public key the official 3.4.3.54261 client validates against, the client rejects `SMSG_ENTER_ENCRYPTED_MODE` and the connection drops before any encrypted packet flows. **High priority to validate against an actual successful client handshake capture or against the C++ reference key file.** Sub-task `#CRYPTO.11`.
2. **✅ Username/password normalisation documented and centralised** — Rust now uses `wow_core::utf8_to_upper_only_latin_like_cpp` inside Grunt SRP6 `compute_x` and `compute_client_evidence`. This intentionally keeps normalisation inside the Rust crypto API, while using the exact C++ Basic-Latin-only semantics (`Util.cpp:795-804`, `Util.h:124-130`, `Util.h:280-283`). The earlier "Latin-1 supplement break" finding was stale: C++ does **not** uppercase `é`, `À`, `ß`, or `ÿ` in this helper.
3. **⚠️ No cross-impl test vectors** — every test in `wow-crypto/` is internal round-trip. Combined with finding #1, there is no offline check that would catch a regression in Rust's nonce/HMAC byte order short of a live login. Recommend ingesting the existing C++ `bot debug` HMAC dump (already produced by `WorldSocket.cpp:755`) as a hardcoded vector. Sub-tasks `#CRYPTO.1`, `#CRYPTO.5`, `#CRYPTO.7`.
4. **⚠️ Counter overflow** — both Rust and C++ silently UB on `u64`/`uint64` overflow. Practically unreachable but the doc claimed this would warn at 2^60. It does not. Sub-task `#CRYPTO.6` open.

No ❌ blocker found in the wire-protocol path itself: SRP6 constants, AES-GCM nonce, tag truncation, HMAC keying order, KDF seeds, and Ed25519ctx signing algorithm all match the C++ canonical implementation byte-for-byte.

### 13.3 Recommended action — priority queue

1. **`#CRYPTO.11` — Ed25519 key sourcing (HIGH).** Compare `ENTER_ENCRYPTED_MODE_PRIVATE_KEY` at `world_socket.rs:81-86` against the C++ `EnterEncryptedModeSigner` PEM (or the corresponding public key shipped in the 3.4.3.54261 client). Treat as login-blocker until verified. If public keys diverge, load from `build_info` like C++ does.
2. **`#CRYPTO.1` + `#CRYPTO.5` + `#CRYPTO.7` — vector tests (HIGH).** Capture one HMAC chain (digest, session_key, encrypt_key) from a working C++ login (the `[DEBUG] WorldSocket _encryptKey` log line at `WorldSocket.cpp:755` gives one for free) and bake it into a `#[test]` against the Rust path with the same inputs. Same for an SRP6 `(salt, verifier)` pair from `account` table.
3. **`#CRYPTO.6` — counter overflow detection (LOW).** Add a `tracing::warn!` once `server_counter` or `client_counter` crosses `1u64 << 60`.
4. **`#CRYPTO.14` — fuzz invalid `A` (MEDIUM).** Already partially covered by `verify_client_proof`'s `(A % N).is_zero()` check at `srp6.rs:320`, but not by tests.
5. Sub-tasks `#CRYPTO.13` (Argon2) and `#CRYPTO.10` (RSA build_info) remain deferrable as long as 2FA is off and the build_info path is not exercised.

Suspicions in §8 about KDF labels `"ServerToClient\0"` / `"ClientToServer\0"` and BE-vs-LE nonce format are **not divergences** — they were misremembered from a different (BNet REST / modern Battle.net) protocol. The real direction binding is the IV magic suffix; the audit confirms Rust matches C++ on that mechanism.

### 13.4 Justifying the index status

The `✅ done (~95%)` claim in the document header is **mostly justified** for the wire-protocol-critical primitives (SRP6 algorithm + AES-GCM + HMAC-SHA256 KDF chain + Ed25519ctx framing all line up with C++). The remaining headroom corresponds to Ed25519 key sourcing, missing cross-impl test vectors, and counter-overflow telemetry. None of those is a known silent-divergence wire bug, but `#CRYPTO.11` remains a real login-risk until the configured C++ signer/public key is verified. Recommendation: keep `✅ done (~95%)` but resolve `#CRYPTO.11` before declaring 100%.

---

*Template version: 1.0 (2026-05-01).*
