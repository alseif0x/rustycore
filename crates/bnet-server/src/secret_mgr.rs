//! Battle.net SecretMgr bootstrap.
//!
//! Mirrors the `SECRET_OWNER_BNETSERVER` path in TrinityCore `SecretMgr`.

use aes::Aes128;
use aes_gcm::aead::generic_array::typenum::U12;
use aes_gcm::aead::{AeadInPlace, KeyInit};
use aes_gcm::{AesGcm, Nonce, Tag};
use anyhow::{Context, Result, anyhow, bail};
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::{Algorithm, Argon2, Params, Version};
use num_bigint::BigUint;
use rand::RngCore;
use wow_database::{LoginDatabase, LoginStatements, SqlTransaction};

const SECRET_TOTP_MASTER_KEY: u32 = 0;
const TOTP_MASTER_SECRET_BITS: usize = 128;
const ARGON2_HASH_LEN: usize = 16;
const ARGON2_ITERATIONS: u32 = 10;
const ARGON2_MEMORY_KIB: u32 = 1 << 17;
const ARGON2_PARALLELISM: u32 = 1;
const AES_KEY_BYTES: usize = 16;
const AES_IV_BYTES: usize = 12;
const AES_TAG_BYTES: usize = 12;

type TcAes128Gcm = AesGcm<Aes128, U12, U12>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretOwner {
    BnetServer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SecretMgrReport {
    pub totp_master_secret_present: bool,
    pub transitioned: bool,
}

pub async fn initialize_secret_mgr_like_cpp(
    login_db: &LoginDatabase,
    owner: SecretOwner,
) -> Result<SecretMgrReport> {
    match owner {
        SecretOwner::BnetServer => initialize_totp_master_secret_like_cpp(login_db).await,
    }
}

async fn initialize_totp_master_secret_like_cpp(
    login_db: &LoginDatabase,
) -> Result<SecretMgrReport> {
    let old_digest = load_secret_digest_like_cpp(login_db).await?;
    let current_value = read_secret_from_config_like_cpp("TOTPMasterSecret")?;

    let digest_matches_current = match (&old_digest, &current_value) {
        (None, None) => true,
        (Some(digest), Some(current)) => verify_secret_digest_like_cpp(current, digest)?,
        _ => false,
    };

    if !digest_matches_current {
        let old_secret = if old_digest.is_some() {
            let maybe_old = read_secret_from_config_like_cpp("TOTPOldMasterSecret")?;
            if let Some(old_secret) = &maybe_old {
                if !verify_secret_digest_like_cpp(old_secret, old_digest.as_deref().unwrap())? {
                    bail!(
                        "Invalid value for 'TOTPOldMasterSecret' specified - this is not actually the secret previously used in your auth DB."
                    );
                }
            }
            maybe_old
        } else {
            None
        };

        attempt_totp_transition_like_cpp(
            login_db,
            current_value.as_ref(),
            old_secret.as_ref(),
            old_digest.is_some(),
        )
        .await?;

        tracing::info!("Successfully transitioned database to new 'TOTPMasterSecret' value.");
        return Ok(SecretMgrReport {
            totp_master_secret_present: current_value.is_some(),
            transitioned: true,
        });
    }

    Ok(SecretMgrReport {
        totp_master_secret_present: current_value.is_some(),
        transitioned: false,
    })
}

async fn load_secret_digest_like_cpp(login_db: &LoginDatabase) -> Result<Option<String>> {
    let mut stmt = login_db.prepare(LoginStatements::SEL_SECRET_DIGEST);
    stmt.set_u32(0, SECRET_TOTP_MASTER_KEY);
    let result = login_db
        .query(&stmt)
        .await
        .context("failed to query secret_digest")?;
    if result.is_empty() {
        Ok(None)
    } else {
        Ok(Some(result.read_string(0)))
    }
}

async fn attempt_totp_transition_like_cpp(
    login_db: &LoginDatabase,
    new_secret: Option<&TcSecret>,
    old_secret: Option<&TcSecret>,
    had_old_secret: bool,
) -> Result<()> {
    let mut tx = SqlTransaction::new();

    let mut result = login_db
        .direct_query("SELECT id, totp_secret FROM account")
        .await
        .context("failed to query account TOTP secrets")?;
    if !result.is_empty() {
        loop {
            if !result.is_null(1) {
                let account_id: u32 = result.read(0);
                let mut totp_secret: Vec<u8> = result.try_read(1).unwrap_or_default();

                if had_old_secret {
                    let old_secret = old_secret.ok_or_else(|| {
                        anyhow!(
                            "Cannot decrypt old TOTP tokens - add config key 'TOTPOldMasterSecret' to authserver.conf!"
                        )
                    })?;

                    decrypt_totp_secret_like_cpp(&mut totp_secret, old_secret).with_context(
                        || {
                            "Cannot decrypt old TOTP tokens - value of 'TOTPOldMasterSecret' is incorrect for some users!"
                        },
                    )?;
                }

                if let Some(new_secret) = new_secret {
                    encrypt_totp_secret_like_cpp(&mut totp_secret, new_secret)?;
                }

                let mut update = login_db.prepare(LoginStatements::UPD_ACCOUNT_TOTP_SECRET);
                update.set_bytes(0, totp_secret);
                update.set_u32(1, account_id);
                tx.append(update);
            }

            if !result.next_row() {
                break;
            }
        }
    }

    if had_old_secret {
        let mut delete = login_db.prepare(LoginStatements::DEL_SECRET_DIGEST);
        delete.set_u32(0, SECRET_TOTP_MASTER_KEY);
        tx.append(delete);
    }

    if let Some(new_secret) = new_secret {
        let digest = hash_secret_digest_like_cpp(new_secret)?;
        let mut insert = login_db.prepare(LoginStatements::INS_SECRET_DIGEST);
        insert.set_u32(0, SECRET_TOTP_MASTER_KEY);
        insert.set_string(1, digest);
        tx.append(insert);
    }

    login_db
        .commit_transaction(tx)
        .await
        .context("failed to commit SecretMgr transition")?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TcSecret {
    value: BigUint,
}

impl TcSecret {
    fn as_hex_str_like_cpp(&self) -> String {
        self.value.to_str_radix(16).to_uppercase()
    }

    fn to_aes_key_like_cpp(&self) -> [u8; AES_KEY_BYTES] {
        let mut key = [0u8; AES_KEY_BYTES];
        let bytes = self.value.to_bytes_le();
        let copy_len = bytes.len().min(AES_KEY_BYTES);
        key[..copy_len].copy_from_slice(&bytes[..copy_len]);
        key
    }
}

fn read_secret_from_config_like_cpp(key: &str) -> Result<Option<TcSecret>> {
    let raw = wow_config::get_string_default(key, "");
    parse_secret_config_value_like_cpp(key, &raw)
}

fn parse_secret_config_value_like_cpp(key: &str, raw: &str) -> Result<Option<TcSecret>> {
    if raw.is_empty() {
        return Ok(None);
    }

    let Some(mut value) = BigUint::parse_bytes(raw.as_bytes(), 16) else {
        bail!(
            "Invalid value for '{key}' - specify a hexadecimal integer of up to 128 bits with no prefix."
        );
    };

    let threshold = BigUint::from(1u32) << TOTP_MASTER_SECRET_BITS;
    if value >= threshold {
        tracing::error!(
            "Value for '{}' is out of bounds (should be an integer of up to 128 bits with no prefix). Truncated to 128 bits.",
            key
        );
        value %= threshold;
    }

    Ok(Some(TcSecret { value }))
}

fn verify_secret_digest_like_cpp(secret: &TcSecret, digest: &str) -> Result<bool> {
    let parsed = PasswordHash::new(digest)
        .map_err(|error| anyhow!("invalid stored secret digest: {error}"))?;
    let params = tc_argon2_like_cpp()?;
    Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .verify_password(secret.as_hex_str_like_cpp().as_bytes(), &parsed)
        .is_ok())
}

fn hash_secret_digest_like_cpp(secret: &TcSecret) -> Result<String> {
    let mut salt_bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut salt_bytes);
    let salt = SaltString::encode_b64(&salt_bytes)
        .map_err(|error| anyhow!("failed to encode Argon2 salt: {error}"))?;
    let params = tc_argon2_like_cpp()?;
    let hash = Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password(secret.as_hex_str_like_cpp().as_bytes(), &salt)
        .map_err(|error| anyhow!("failed to hash secret digest: {error}"))?;
    Ok(hash.to_string())
}

fn tc_argon2_like_cpp() -> Result<Params> {
    Params::new(
        ARGON2_MEMORY_KIB,
        ARGON2_ITERATIONS,
        ARGON2_PARALLELISM,
        Some(ARGON2_HASH_LEN),
    )
    .map_err(|error| anyhow!("invalid Argon2 parameters: {error}"))
}

fn encrypt_totp_secret_like_cpp(data: &mut Vec<u8>, secret: &TcSecret) -> Result<()> {
    let key = secret.to_aes_key_like_cpp();
    let cipher = TcAes128Gcm::new_from_slice(&key).context("failed to initialize AES-GCM")?;
    let mut iv = [0u8; AES_IV_BYTES];
    rand::thread_rng().fill_bytes(&mut iv);
    let tag = cipher
        .encrypt_in_place_detached(Nonce::<U12>::from_slice(&iv), b"", data)
        .map_err(|error| anyhow!("failed to encrypt TOTP secret: {error}"))?;
    data.extend_from_slice(&iv);
    data.extend_from_slice(tag.as_slice());
    Ok(())
}

fn decrypt_totp_secret_like_cpp(data: &mut Vec<u8>, secret: &TcSecret) -> Result<()> {
    if data.len() < AES_IV_BYTES + AES_TAG_BYTES {
        bail!("encrypted TOTP secret is shorter than IV+tag");
    }

    let tag_offset = data.len() - AES_TAG_BYTES;
    let iv_offset = tag_offset - AES_IV_BYTES;
    let tag_bytes: Vec<u8> = data.drain(tag_offset..).collect();
    let iv_bytes: Vec<u8> = data.drain(iv_offset..).collect();
    let key = secret.to_aes_key_like_cpp();
    let cipher = TcAes128Gcm::new_from_slice(&key).context("failed to initialize AES-GCM")?;
    cipher
        .decrypt_in_place_detached(
            Nonce::<U12>::from_slice(&iv_bytes),
            b"",
            data,
            Tag::<U12>::from_slice(&tag_bytes),
        )
        .map_err(|error| anyhow!("failed to decrypt TOTP secret: {error}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_config_empty_is_not_present_like_cpp() {
        assert!(
            parse_secret_config_value_like_cpp("TOTPMasterSecret", "")
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn secret_config_parses_hex_and_truncates_to_128_bits_like_cpp() {
        let parsed = parse_secret_config_value_like_cpp(
            "TOTPMasterSecret",
            "10000000000000000000000000000000f",
        )
        .unwrap()
        .expect("secret");
        assert_eq!(parsed.as_hex_str_like_cpp(), "F");
    }

    #[test]
    fn secret_aes_key_uses_little_endian_bignum_bytes_like_cpp() {
        let parsed = parse_secret_config_value_like_cpp("TOTPMasterSecret", "010203")
            .unwrap()
            .expect("secret");
        let key = parsed.to_aes_key_like_cpp();
        assert_eq!(&key[..3], &[0x03, 0x02, 0x01]);
        assert!(key[3..].iter().all(|byte| *byte == 0));
    }

    #[test]
    fn totp_secret_encrypt_appends_iv_and_tag_and_decrypts_like_cpp() {
        let secret = parse_secret_config_value_like_cpp(
            "TOTPMasterSecret",
            "00112233445566778899AABBCCDDEEFF",
        )
        .unwrap()
        .expect("secret");
        let mut data = b"totp-secret".to_vec();
        let plain = data.clone();

        encrypt_totp_secret_like_cpp(&mut data, &secret).expect("encrypt");
        assert_eq!(data.len(), plain.len() + AES_IV_BYTES + AES_TAG_BYTES);
        assert_ne!(&data[..plain.len()], plain.as_slice());

        decrypt_totp_secret_like_cpp(&mut data, &secret).expect("decrypt");
        assert_eq!(data, plain);
    }

    #[test]
    fn secret_digest_hash_verifies_like_cpp_params() {
        let secret = parse_secret_config_value_like_cpp("TOTPMasterSecret", "1234")
            .unwrap()
            .expect("secret");
        let digest = hash_secret_digest_like_cpp(&secret).expect("hash");
        assert!(verify_secret_digest_like_cpp(&secret, &digest).expect("verify"));
    }
}
