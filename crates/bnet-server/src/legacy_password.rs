//! Legacy Battle.net password hash migration.
//!
//! Mirrors TrinityCore `LoginRESTService::MigrateLegacyPasswordHashes()`.

use anyhow::{Context, Result, bail};
use wow_crypto::{SrpVersion, compute_bnet_v1_verifier_from_legacy_sha_hash, generate_bnet_salt};
use wow_database::{LoginDatabase, LoginStatements, PreparedStatement, SqlResult, SqlTransaction};

const HAS_SHA_PASS_HASH_COLUMN_SQL: &str = concat!(
    "SELECT 1 FROM information_schema.COLUMNS ",
    "WHERE TABLE_SCHEMA = SCHEMA() ",
    "AND TABLE_NAME = 'battlenet_accounts' ",
    "AND COLUMN_NAME = 'sha_pass_hash'",
);

const SELECT_LEGACY_PASSWORD_HASHES_SQL: &str = concat!(
    "SELECT id, sha_pass_hash, ",
    "IF((salt IS null) OR (verifier IS null), 0, 1) AS shouldWarn ",
    "FROM battlenet_accounts ",
    "WHERE sha_pass_hash != DEFAULT(sha_pass_hash) OR salt IS NULL OR verifier IS NULL",
);

const RESET_LEGACY_PASSWORD_HASH_SQL: &str =
    "UPDATE battlenet_accounts SET sha_pass_hash = DEFAULT(sha_pass_hash) WHERE id = ?";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LegacyPasswordMigrationReport {
    pub column_present: bool,
    pub updated_accounts: u32,
    pub warned_external_tool: bool,
}

pub async fn migrate_legacy_password_hashes_like_cpp(
    login_db: &LoginDatabase,
) -> Result<LegacyPasswordMigrationReport> {
    if !legacy_sha_pass_hash_column_exists_like_cpp(login_db).await? {
        return Ok(LegacyPasswordMigrationReport {
            column_present: false,
            updated_accounts: 0,
            warned_external_tool: false,
        });
    }

    tracing::info!("Updating password hashes...");

    let mut result = login_db
        .direct_query(SELECT_LEGACY_PASSWORD_HASHES_SQL)
        .await
        .context("failed to query legacy Battle.net password hashes")?;

    if result.is_empty() {
        tracing::info!("No password hashes to update");
        return Ok(LegacyPasswordMigrationReport {
            column_present: true,
            updated_accounts: 0,
            warned_external_tool: false,
        });
    }

    let mut warned_external_tool = false;
    let mut updated_accounts = 0;
    let mut tx = SqlTransaction::new();

    loop {
        let account_id: u32 = result.read(0);
        let sha_pass_hash = result.read_string(1);
        let should_warn = read_should_warn_like_cpp(&result);
        let salt = generate_bnet_salt();
        let legacy_hash = parse_sha_pass_hash_like_cpp(&sha_pass_hash)
            .with_context(|| format!("invalid sha_pass_hash for battlenet account {account_id}"))?;
        let verifier = compute_bnet_v1_verifier_from_legacy_sha_hash(&salt, &legacy_hash);

        if should_warn && !warned_external_tool {
            warned_external_tool = true;
            tracing::warn!(
                "You appear to be using an outdated external account management tool. Update your external tool."
            );
        }

        let mut update_logon = login_db.prepare(LoginStatements::UPD_BNET_LOGON);
        update_logon.set_i8(0, SrpVersion::V1 as i8);
        update_logon.set_bytes(1, salt.to_vec());
        update_logon.set_bytes(2, verifier);
        update_logon.set_u32(3, account_id);
        tx.append(update_logon);

        let mut reset_legacy_hash = PreparedStatement::new(RESET_LEGACY_PASSWORD_HASH_SQL);
        reset_legacy_hash.set_u32(0, account_id);
        tx.append(reset_legacy_hash);

        if tx.len() >= 10_000 {
            login_db
                .commit_transaction(std::mem::take(&mut tx))
                .await
                .context("failed to commit legacy Battle.net password hash batch")?;
        }

        updated_accounts += 1;
        if !result.next_row() {
            break;
        }
    }

    if !tx.is_empty() {
        login_db
            .commit_transaction(tx)
            .await
            .context("failed to commit legacy Battle.net password hash batch")?;
    }

    tracing::info!("{updated_accounts} password hashes updated");
    Ok(LegacyPasswordMigrationReport {
        column_present: true,
        updated_accounts,
        warned_external_tool,
    })
}

async fn legacy_sha_pass_hash_column_exists_like_cpp(login_db: &LoginDatabase) -> Result<bool> {
    let result = login_db
        .direct_query(HAS_SHA_PASS_HASH_COLUMN_SQL)
        .await
        .context("failed to inspect battlenet_accounts.sha_pass_hash column")?;
    Ok(!result.is_empty())
}

fn parse_sha_pass_hash_like_cpp(value: &str) -> Result<[u8; 32]> {
    if value.len() != 64 {
        bail!("expected 64 hexadecimal characters, got {}", value.len());
    }

    let mut be = [0u8; 32];
    for (index, chunk) in value.as_bytes().chunks_exact(2).enumerate() {
        be[index] = decode_hex_byte(chunk).context("expected hexadecimal sha_pass_hash")?;
    }
    be.reverse();
    Ok(be)
}

fn read_should_warn_like_cpp(result: &SqlResult) -> bool {
    result
        .try_read::<i64>(2)
        .map(|value| value != 0)
        .or_else(|| result.try_read::<i32>(2).map(|value| value != 0))
        .or_else(|| result.try_read::<u64>(2).map(|value| value != 0))
        .or_else(|| result.try_read::<u32>(2).map(|value| value != 0))
        .or_else(|| result.try_read::<i8>(2).map(|value| value != 0))
        .or_else(|| result.try_read::<u8>(2).map(|value| value != 0))
        .unwrap_or(false)
}

fn decode_hex_byte(chunk: &[u8]) -> Result<u8> {
    Ok((decode_hex_nibble(chunk[0])? << 4) | decode_hex_nibble(chunk[1])?)
}

fn decode_hex_nibble(byte: u8) -> Result<u8> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => bail!("invalid hexadecimal digit"),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_sha_pass_hash_like_cpp;

    #[test]
    fn legacy_sha_pass_hash_parser_reverses_hex_bytes_like_cpp() {
        let parsed = parse_sha_pass_hash_like_cpp(
            "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
        )
        .expect("valid hash");

        assert_eq!(parsed[0], 0x1f);
        assert_eq!(parsed[31], 0x00);
    }

    #[test]
    fn legacy_sha_pass_hash_parser_rejects_non_cpp_length() {
        let error = parse_sha_pass_hash_like_cpp("").expect_err("empty hash must fail");
        assert!(
            error
                .to_string()
                .contains("expected 64 hexadecimal characters")
        );
    }

    #[test]
    fn legacy_sha_pass_hash_parser_rejects_non_hex() {
        let error = parse_sha_pass_hash_like_cpp(
            "zz0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
        )
        .expect_err("non-hex hash must fail");
        assert!(error.to_string().contains("expected hexadecimal"));
    }
}
