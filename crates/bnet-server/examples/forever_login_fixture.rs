//! Create the disposable Auth-only fixture used by the WoW Forever login smoke.
//!
//! This deliberately refuses to touch a non-local or non-empty database. Run it
//! with exactly a BNet config path and a private file containing the BNet password.

use anyhow::{Context, Result, bail};
use rand::RngCore;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use wow_config::{DatabaseInfo, parse_database_info};
use wow_crypto::{SrpVersion, compute_bnet_verifier, srp_username};
use wow_database::migration::{self, DatabaseKind, MigrationManifest};
use wow_database::{
    LoginDatabase, PreparedStatement, SqlTransaction, build_connection_string_with_ssl_like_cpp,
};

const BIND_IP: &str = "127.0.0.1";
const DB_HOST: &str = "127.0.0.1";
const DB_PORT: &str = "13316";
const AUTH_DATABASE: &str = "auth_forever_70170";
const BNET_EMAIL: &str = "FOREVER@LOCAL.TEST";
const GAME_ACCOUNT_ID: u32 = 1;
const BNET_ACCOUNT_ID: u32 = 1;
const BUILD: u32 = 70170;

#[tokio::main]
async fn main() -> Result<()> {
    let (config_path, password_path) = parse_args()?;
    wow_config::load_config(
        config_path
            .to_str()
            .context("configuration path is not valid UTF-8")?,
    )
    .context("failed to load BNet configuration")?;

    let raw_login_info = wow_config::get_string_default("LoginDatabaseInfo", "");
    if raw_login_info.is_empty() {
        bail!("LoginDatabaseInfo is required in the fixture configuration");
    }
    let login_info = parse_database_info("LoginDatabaseInfo", &raw_login_info)
        .context("invalid LoginDatabaseInfo")?;
    validate_local_target(
        &login_info,
        &wow_config::get_string_default("BindIP", "0.0.0.0"),
    )?;

    let password = read_password(&password_path)?;
    let connection = build_connection_string_with_ssl_like_cpp(
        &login_info.host,
        &login_info.port_or_socket,
        &login_info.username,
        &login_info.password,
        &login_info.database,
        login_info.ssl,
    );
    let login_db = LoginDatabase::open_with_pool_size(&connection, 4)
        .await
        .context("failed to connect to the isolated Auth database")?;

    require_empty_tables(&login_db).await?;

    // bundled_manifest is the runtime identity. Applying SQL needs the source
    // manifest because migrate_database reads its migration files from disk.
    let runtime_manifest = migration::bundled_manifest()?;
    let source_manifest = source_manifest()?;
    migration::migrate_database(login_db.pool(), &source_manifest, DatabaseKind::Auth)
        .await
        .context("failed to apply Auth migrations")?;
    migration::validate_runtime_schema(login_db.pool(), &runtime_manifest, DatabaseKind::Auth)
        .await
        .context("Auth schema is not runtime-compatible after migration")?;

    // Re-check after the migration fence, immediately before the seed transaction.
    require_empty_tables(&login_db).await?;
    seed_fixture(&login_db, &password).await?;

    println!("Provisioned the isolated 1.60.1.70170 Auth login fixture.");
    Ok(())
}

fn parse_args() -> Result<(PathBuf, PathBuf)> {
    let mut args = env::args_os();
    let _program = args.next();
    let config = args
        .next()
        .map(PathBuf::from)
        .context("usage: forever_login_fixture <bnet-config> <password-file>")?;
    let password = args
        .next()
        .map(PathBuf::from)
        .context("usage: forever_login_fixture <bnet-config> <password-file>")?;
    if args.next().is_some() {
        bail!("usage: forever_login_fixture <bnet-config> <password-file>");
    }
    Ok((config, password))
}

fn read_password(path: &Path) -> Result<String> {
    let value = fs::read_to_string(path).context("failed to read the password file")?;
    validate_password_text(&value)
}

fn validate_password_text(value: &str) -> Result<String> {
    let value = value.trim_end_matches(['\r', '\n']).to_owned();
    if value.is_empty() {
        bail!("password file is empty");
    }
    if value.chars().count() > 128 {
        bail!("password exceeds the Battle.net 128-character limit");
    }
    Ok(value)
}

fn validate_local_target(info: &DatabaseInfo, bind_ip: &str) -> Result<()> {
    if info.host != DB_HOST || info.port_or_socket != DB_PORT || info.database != AUTH_DATABASE {
        bail!(
            "fixture requires LoginDatabaseInfo host={DB_HOST}, port={DB_PORT}, database={AUTH_DATABASE}"
        );
    }
    if bind_ip != BIND_IP {
        bail!("fixture requires BindIP={BIND_IP}");
    }
    Ok(())
}

fn source_manifest() -> Result<MigrationManifest> {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../database/migrations/manifest.toml");
    MigrationManifest::load(&path)
}

async fn require_empty_tables(login_db: &LoginDatabase) -> Result<()> {
    for table in ["account", "battlenet_accounts", "realmlist"] {
        let result = login_db
            .direct_query(&format!("SELECT COUNT(*) FROM `{table}`"))
            .await
            .with_context(|| format!("failed to inspect Auth table {table}"))?;
        let count = result
            .try_read::<u64>(0)
            .or_else(|| result.try_read::<i64>(0).map(|value| value as u64))
            .context("Auth table count was not numeric")?;
        if count != 0 {
            bail!("refusing to overwrite non-empty Auth table {table}");
        }
    }
    Ok(())
}

async fn seed_fixture(login_db: &LoginDatabase, password: &str) -> Result<()> {
    let bnet_salt = random_32();
    let bnet_username = srp_username(BNET_EMAIL);
    let bnet_verifier = compute_bnet_verifier(SrpVersion::V2, &bnet_username, password, &bnet_salt);
    let game_salt = random_32();
    let game_verifier = random_32();
    let mut tx = SqlTransaction::new();

    // C++ reference: src/server/game/Accounts/BattlenetAccountMgr.cpp:34-68
    // creates the BNet identity first, then its numeric game-account #1.
    let mut bnet = PreparedStatement::new(
        "INSERT INTO battlenet_accounts (id, email, srp_version, salt, verifier) VALUES (?, ?, ?, ?, ?)",
    );
    bnet.set_u32(0, BNET_ACCOUNT_ID);
    bnet.set_string(1, BNET_EMAIL);
    bnet.set_i8(2, SrpVersion::V2 as i8);
    bnet.set_bytes(3, bnet_salt.to_vec());
    bnet.set_bytes(4, bnet_verifier);
    tx.append_expect_rows_affected(bnet, 1);

    let mut game = PreparedStatement::new(
        "INSERT INTO account (id, username, salt, verifier, reg_mail, email, battlenet_account, battlenet_index, expansion, client_build) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    );
    game.set_u32(0, GAME_ACCOUNT_ID);
    game.set_string(1, "1#1");
    game.set_bytes(2, game_salt.to_vec());
    game.set_bytes(3, game_verifier.to_vec());
    game.set_string(4, BNET_EMAIL);
    game.set_string(5, BNET_EMAIL);
    game.set_u32(6, BNET_ACCOUNT_ID);
    game.set_u8(7, 1);
    game.set_u8(8, 2);
    game.set_u32(9, BUILD);
    tx.append_expect_rows_affected(game, 1);

    // C++ reference: src/server/shared/Realm/RealmList.cpp:64-94 and 98-114.
    // Seeds stay NULL because this fixture stops before world AuthSession.
    let mut build = PreparedStatement::new(
        "INSERT INTO build_info (build, majorVersion, minorVersion, bugfixVersion, hotfixVersion, win64AuthSeed, mac64AuthSeed) VALUES (?, ?, ?, ?, ?, ?, ?)",
    );
    build.set_u32(0, BUILD);
    build.set_u32(1, 1);
    build.set_u32(2, 60);
    build.set_u32(3, 1);
    build.set_null(4);
    build.set_null(5);
    build.set_null(6);
    tx.append_expect_rows_affected(build, 1);

    let mut realm = PreparedStatement::new(
        "INSERT INTO realmlist (id, name, address, localAddress, localSubnetMask, port, icon, flag, timezone, allowedSecurityLevel, population, gamebuild, Region, Battlegroup) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    );
    realm.set_u32(0, 1);
    realm.set_string(1, "RustyCore Forever - Login Test");
    realm.set_string(2, "127.0.0.1");
    realm.set_string(3, "127.0.0.1");
    realm.set_string(4, "255.255.255.0");
    realm.set_u16(5, 18085);
    realm.set_u8(6, 0);
    realm.set_u8(7, 2);
    realm.set_u8(8, 1);
    realm.set_u8(9, 0);
    realm.set_f32(10, 0.0);
    realm.set_u32(11, BUILD);
    realm.set_u8(12, 2);
    realm.set_u8(13, 1);
    tx.append_expect_rows_affected(realm, 1);

    login_db
        .commit_transaction(tx)
        .await
        .context("failed to commit the isolated Auth fixture")?;
    Ok(())
}

fn random_32() -> [u8; 32] {
    let mut value = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut value);
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_removes_only_line_endings() {
        assert_eq!(validate_password_text(" pass \r\n").unwrap(), " pass ");
    }

    #[test]
    fn empty_password_is_rejected() {
        assert!(validate_password_text("\r\n").is_err());
    }

    #[test]
    fn local_guard_rejects_non_local_database() {
        let info = DatabaseInfo::new("127.0.0.2", 13316, "user", "password", AUTH_DATABASE);
        assert!(validate_local_target(&info, BIND_IP).is_err());
    }
}
