//! Target-specific mandatory resources, never the legacy startup catalogs.
use super::character_capture::CharacterCapture;
use anyhow::{Context, Result, bail, ensure};
use std::{env, fs, path::Path, sync::Arc};
use wow_data::forever_character_ids::{ForeverAchievementIds, ForeverCharacterIds};
use wow_data::forever_hotfix::{ForeverHotfixCatalog, ForeverTactKeys, TACT_KEY_TABLE_HASH};
use wow_data::{HotfixBlobCache, hotfix_cache::HotfixRecordStatus};
use wow_database::{
    CharacterDatabase, HotfixDatabase, LoginDatabase,
    MariaDbHotfixDeliveryMetadataPersistenceAdapterLikeCpp, WorldDatabase,
    build_connection_string_with_ssl_like_cpp,
    forever::{ForeverAvailabilityRepository, ForeverSessionRepository},
    forever_hotfix::ForeverHotfixRepository,
};
use wow_persistence::forever::{AvailabilityRepository, SessionRepository};
use wow_persistence::{
    HotfixDeliveryMetadataLoadOutcomeLikeCpp, HotfixDeliveryMetadataPersistencePortLikeCpp,
};
use wow_world::forever::{CharacterCatalog, InitializationPolicy};

pub(super) struct Runtime {
    pub auth: Arc<LoginDatabase>,
    pub session_repository: Arc<dyn SessionRepository>,
    pub catalog: CharacterCatalog,
    pub hotfixes: Arc<ForeverHotfixCatalog>,
    pub build_key: [u8; 16],
    pub policy: InitializationPolicy,
    pub region_group: i32,
    pub character_idle_timeout: std::time::Duration,
    pub character_capture: Option<CharacterCapture>,
}

pub(super) async fn load() -> Result<Runtime> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if !matches!(args.len(), 4 | 6) || args[0] != "--ack-isolated-forever" {
        bail!(
            "usage: --ack-isolated-forever <bnet-config> <private-build-key-file> <private-target-db2-directory> [--ack-private-character-create-capture <new-private-file>]"
        );
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/forever-login")
        .canonicalize()?;
    for path in &args[1..4] {
        ensure!(
            Path::new(path).canonicalize()?.starts_with(&root),
            "outside isolated runtime"
        );
    }
    let character_capture = if args.len() == 6 {
        ensure!(
            args[4] == "--ack-private-character-create-capture",
            "unknown opt-in"
        );
        Some(CharacterCapture::prepare(&root, Path::new(&args[5]))?)
    } else {
        None
    };
    let key_path = Path::new(&args[2]);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        ensure!(
            fs::metadata(key_path)?.permissions().mode() & 0o077 == 0,
            "key is not private"
        );
    }
    let build_key = fs::read(key_path)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("build key length"))?;
    wow_config::load_config(args[1].to_str().context("config path")?)?;
    let info = wow_config::parse_database_info(
        "LoginDatabaseInfo",
        &wow_config::get_string_default("LoginDatabaseInfo", ""),
    )?;
    ensure!(
        info.host == "127.0.0.1"
            && info.port_or_socket == "13316"
            && info.database == "auth_forever_70170"
            && wow_config::get_string_default("BindIP", "") == "127.0.0.1",
        "not isolated databases"
    );
    let url = |database| {
        build_connection_string_with_ssl_like_cpp(
            &info.host,
            &info.port_or_socket,
            &info.username,
            &info.password,
            database,
            info.ssl,
        )
    };
    let auth = Arc::new(LoginDatabase::open_with_pool_size(&url(&info.database), 1).await?);
    let characters = Arc::new(
        CharacterDatabase::open_with_pool_size(&url("characters_forever_70170"), 1).await?,
    );
    let world =
        Arc::new(WorldDatabase::open_with_pool_size(&url("world_forever_70170_02245"), 1).await?);
    let hotfix =
        Arc::new(HotfixDatabase::open_with_pool_size(&url("hotfixes_forever_70170"), 1).await?);
    let ids = ForeverCharacterIds::load(Path::new(&args[3]))?;
    let achievements = ForeverAchievementIds::load(Path::new(&args[3]))?;
    let rows = ForeverAvailabilityRepository::new(world)
        .load_availability()
        .await
        .map_err(|_| anyhow::anyhow!("availability query failed"))?;
    let catalog = CharacterCatalog::load(&ids, &achievements, rows).map_err(anyhow::Error::msg)?;
    let tact_keys = ForeverTactKeys::load(Path::new(&args[3]))?;
    let overlays = ForeverHotfixRepository::new(hotfix.clone())
        .load_tact_key_overlays()
        .await
        .map_err(|_| anyhow::anyhow!("TactKey overlay query failed"))?;
    let tact_keys = tact_keys.with_overlays(
        overlays.official.into_iter().map(|row| (row.id, row.key)),
        overlays.custom.into_iter().map(|row| (row.id, row.key)),
    )?;
    let hotfix_adapter = MariaDbHotfixDeliveryMetadataPersistenceAdapterLikeCpp::new(hotfix);
    let mut hotfixes = HotfixBlobCache::new();
    hotfixes.register_typed_table(TACT_KEY_TABLE_HASH);
    for name in ["ChrClasses.db2", "ChrRaces.db2"] {
        hotfixes.load_db2(Path::new(&args[3]).join(name))?;
    }
    let blobs = match hotfix_adapter.load_hotfix_blob_rows_like_cpp().await {
        HotfixDeliveryMetadataLoadOutcomeLikeCpp::Loaded(rows) => rows,
        _ => bail!("hotfix blob query failed"),
    };
    hotfixes.apply_hotfix_blob_rows_like_cpp(
        blobs
            .into_iter()
            .map(|row| (row.table_hash, row.record_id, row.locale, row.blob)),
        "esES",
    );
    let rows = match hotfix_adapter.load_hotfix_data_rows_like_cpp().await {
        HotfixDeliveryMetadataLoadOutcomeLikeCpp::Loaded(rows) => rows,
        _ => bail!("hotfix data query failed"),
    };
    for row in &rows {
        ensure!(
            !(row.status == HotfixRecordStatus::Valid as u8
                && hotfixes.has_table(row.table_hash)
                && row.table_hash != TACT_KEY_TABLE_HASH),
            "target typed hotfix serializer required"
        );
        ensure!(row.status <= 4, "invalid hotfix status");
    }
    hotfixes.apply_hotfix_data_rows_like_cpp(
        rows.into_iter().map(|row| {
            (
                row.push_id,
                row.unique_id,
                row.table_hash,
                row.record_id,
                row.status,
            )
        }),
        "esES",
    );
    let rows = match hotfix_adapter
        .load_hotfix_optional_data_rows_like_cpp()
        .await
    {
        HotfixDeliveryMetadataLoadOutcomeLikeCpp::Loaded(rows) => rows,
        _ => bail!("hotfix optional query failed"),
    };
    hotfixes.apply_hotfix_optional_data_rows_like_cpp(
        // DB2Stores.cpp:1866 registers optional TactKey data for BroadcastText,
        // NOT optional data for the TactKey store itself. Its rows are skipped.
        rows.into_iter()
            .filter(|row| row.table_hash != TACT_KEY_TABLE_HASH)
            .map(|row| (row.table_hash, row.record_id, row.locale, row.key, row.data)),
        "esES",
    );
    // Names and expansion are taken from the admitted auth/realm rows later;
    // this policy controls only this isolated target's unimplemented services.
    let policy = InitializationPolicy {
        realm_name: "RustyCore Forever".into(),
        normalized_realm_name: "RustyCoreForever".into(),
        timezone: "Etc/UTC".into(),
        cache_version: 0,
        content_set: wow_config::get_value_default("Realm.CfgContentSetID", 137_i32),
        max_characters: 200,
    };
    // World.cpp:726,1061 and WorldSession::ResetTimeOutTime(false): inactive
    // character selection uses SocketTimeOutTime, not the 30-second probe read.
    // Player-active timeout and queue exemptions belong to the later player port.
    let idle_ms = wow_config::get_value_default("SocketTimeOutTime", 900000_u32);
    ensure!(idle_ms >= 1000, "invalid character idle timeout");
    let character_idle_timeout = std::time::Duration::from_secs(u64::from(idle_ms / 1000));
    let hotfixes = ForeverHotfixCatalog::new(tact_keys, hotfixes)?;
    println!(
        "Forever prerequisites loaded: {} availability races, {} readable achievements, {} hotfix records, {} effective TactKey records; no session admitted yet.",
        catalog.races().len(),
        achievements.available_count(),
        hotfixes.metadata().hotfix_count(),
        hotfixes.tact_key_count()
    );
    Ok(Runtime {
        auth: auth.clone(),
        session_repository: Arc::new(ForeverSessionRepository::new(auth, characters)),
        catalog,
        hotfixes: Arc::new(hotfixes),
        build_key,
        policy,
        character_idle_timeout,
        character_capture,
        region_group: wow_config::get_value_default("Network.EnterEncryptedModeRegionGroup", 0_i32),
    })
}
