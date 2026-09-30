//! Ordered realm startup composition.

use anyhow::Context;
use std::sync::Arc;
use tracing::info;
use std::sync::Mutex;
use wow_database::LoginDatabase;
use crate::{realm_id_like_cpp, clear_online_accounts_like_cpp, verify_world_db_version_like_cpp, set_realm_offline, RealmListSnapshotLikeCpp, update_realm_list_once_like_cpp, spawn_realm_list_update_loop_like_cpp, realms_state_update_delay_secs_like_cpp};

pub(super) struct RealmAvailability {
    pub(super) realm_list_update_handle: Option<tokio::task::JoinHandle<()>>,
    pub(super) realm_list_summary: crate::RealmListRefreshSummaryLikeCpp,
    pub(super) realm_list: crate::SharedRealmListLikeCpp,
    pub(super) realm_id: u16,
}

pub(super) async fn initialize_availability(
    login_db: &wow_database::LoginDatabase,
    char_db: &wow_database::CharacterDatabase,
    world_db: &wow_database::WorldDatabase,
) -> anyhow::Result<RealmAvailability> {
    let realm_id = realm_id_like_cpp()?;
    clear_online_accounts_like_cpp(login_db, char_db, realm_id).await?;
    verify_world_db_version_like_cpp(world_db).await?;
    set_realm_offline(login_db, realm_id).await?;
    let realm_list = Arc::new(Mutex::new(RealmListSnapshotLikeCpp::default()));
    let realm_list_summary = update_realm_list_once_like_cpp(login_db, &realm_list)
        .await
        .context("Failed to initialize RealmList from realmlist")?;
    info!(
        realms = realm_list_summary.realms,
        sub_regions = realm_list_summary.sub_regions,
        added = realm_list_summary.added,
        updated = realm_list_summary.updated,
        removed = realm_list_summary.removed,
        "Initialized RealmList from realmlist like C++"
    );
    let realm_list_update_handle = spawn_realm_list_update_loop_like_cpp(
        LoginDatabase::from_pool(login_db.pool().clone()),
        Arc::clone(&realm_list),
        realms_state_update_delay_secs_like_cpp(),
    );
    Ok(RealmAvailability {
        realm_id,
        realm_list,
        realm_list_summary,
        realm_list_update_handle,
    })
}

pub(super) struct RealmIdentity {
    pub(super) realm_local_address: [u8; 4],
    pub(super) realm_external_address: [u8; 4],
    pub(super) win64_auth_seed: [u8; 16],
    pub(super) realm_build: u32,
    pub(super) realm_names: Arc<Vec<(u32, String, String)>>,
    pub(super) active_realm: crate::RealmListEntryLikeCpp,
}

pub(super) async fn load_identity(
    realm_list: &crate::SharedRealmListLikeCpp,
    realm_id: u16,
    login_db: &wow_database::LoginDatabase,
) -> anyhow::Result<RealmIdentity> {
    let active_realm = crate::load_realm_info_from_snapshot_like_cpp(realm_list, realm_id)?;
    let realm_names = crate::realm_name_records_from_snapshot_like_cpp(realm_list);
    let realm_build = active_realm.build;
    let win64_auth_seed = crate::load_realm_win64_auth_seed_like_cpp(login_db, realm_build).await?;
    info!("Realm {realm_id} build {realm_build}, Win64AuthSeed loaded");

    let realm_external_address = crate::resolve_realm_endpoint_address_like_cpp(
        "address",
        &active_realm.address,
        &active_realm.name,
        u32::from(realm_id),
    )
    .await?;
    let realm_local_address = crate::resolve_realm_endpoint_address_like_cpp(
        "localAddress",
        &active_realm.local_address,
        &active_realm.name,
        u32::from(realm_id),
    )
    .await?;
    info!(
        "Realm addresses: external={}, local={}",
        crate::format_ipv4(realm_external_address),
        crate::format_ipv4(realm_local_address),
    );
    Ok(RealmIdentity {
        active_realm,
        realm_names,
        realm_build,
        win64_auth_seed,
        realm_external_address,
        realm_local_address,
    })
}
