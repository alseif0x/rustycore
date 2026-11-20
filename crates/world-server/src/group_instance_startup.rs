//! Group, terrain and instance-lock startup composition.

use super::*;

pub(super) struct GroupInstanceStartup {
    pub(super) group_registry: Arc<GroupRegistry>,
    pub(super) pending_invites: Arc<PendingInvites>,
    pub(super) represented_group_persistence_adapter: Arc<
        wow_database::represented_group_persistence_adapter::MariaDbRepresentedGroupPersistenceAdapterLikeCpp,
    >,
    pub(super) live_terrain: Arc<wow_world::terrain::LiveTerrainHeights>,
    pub(super) instance_lock_persistence_port:
        Arc<dyn wow_persistence::InstanceLockPersistencePortLikeCpp>,
    pub(super) registered_instance_ids: Vec<u32>,
    pub(super) instance_lock_mgr: Arc<std::sync::RwLock<InstanceLockMgr>>,
}

pub(super) async fn load_group_instance_startup(
    char_db: &Arc<CharacterDatabase>,
    difficulty_store: &wow_data::DifficultyStore,
    map_store: &wow_data::MapStore,
    map_difficulty_store: &wow_data::MapDifficultyStore,
    data_dir: &str,
) -> Result<GroupInstanceStartup> {
    // Shared group registry and pending invites
    let group_registry = Arc::new(GroupRegistry::new());
    let pending_invites = Arc::new(PendingInvites::new());
    let represented_group_persistence_adapter = Arc::new(
        wow_database::represented_group_persistence_adapter::MariaDbRepresentedGroupPersistenceAdapterLikeCpp::new(
            Arc::clone(char_db),
        ),
    );
    let group_load_summary = load_groups_from_character_database_like_cpp(
        represented_group_persistence_adapter.as_ref(),
        group_registry.as_ref(),
        difficulty_store,
    )
    .await
    .context("Failed to load C++ group startup state")?;
    info!(
        "Loaded C++ group startup state: groups={} member-rows={} members={} skipped-groups={} skipped-members={}",
        group_load_summary.loaded_groups,
        group_load_summary.loaded_member_rows,
        group_load_summary.loaded_members,
        group_load_summary.skipped_group_rows,
        group_load_summary.skipped_member_rows,
    );

    // File-backed terrain so the live spawn/respawn path ground-snaps
    // creatures with real `.map` heights (issue #15). DataDir-rooted, lazy;
    // shared by every session and the canonical creature runtime.
    let live_terrain = Arc::new(wow_world::terrain::LiveTerrainHeights::new(&data_dir));
    let instance_lock_persistence_port: Arc<
        dyn wow_persistence::InstanceLockPersistencePortLikeCpp,
    > = Arc::new(
        wow_database::MariaDbInstanceLockPersistenceAdapterLikeCpp::new(Arc::clone(char_db)),
    );
    let (shared_instance_lock_rows, character_instance_lock_rows) =
        match instance_lock_persistence_port.load_all_like_cpp().await {
            wow_persistence::InstanceLockPersistenceLoadOutcomeLikeCpp::Loaded {
                shared_rows,
                character_rows,
            } => (shared_rows, character_rows),
            wow_persistence::InstanceLockPersistenceLoadOutcomeLikeCpp::Failed { reason } => {
                bail!("Failed to load instance locks from character database: {reason}")
            }
        };
    let mut loaded_instance_lock_mgr = InstanceLockMgr::default();
    let instance_lock_load_issues = loaded_instance_lock_mgr.load_from_rows_like_cpp(
        shared_instance_lock_rows,
        character_instance_lock_rows,
        |map_id, difficulty_id| {
            map_db2_entries_from_stores(map_store, map_difficulty_store, map_id, difficulty_id)
        },
    );
    for issue in &instance_lock_load_issues {
        warn!("Instance lock load issue: {issue:?}");
    }
    let instance_lock_stats = loaded_instance_lock_mgr.statistics();
    info!(
        "Loaded instance locks: {} shared instances, {} players, {} issues",
        instance_lock_stats.instance_count,
        instance_lock_stats.player_count,
        instance_lock_load_issues.len()
    );
    let registered_instance_ids = loaded_instance_lock_mgr.registered_instance_ids_like_cpp_order();
    let instance_lock_mgr = Arc::new(std::sync::RwLock::new(loaded_instance_lock_mgr));

    Ok(GroupInstanceStartup {
        group_registry,
        pending_invites,
        represented_group_persistence_adapter,
        live_terrain,
        instance_lock_persistence_port,
        registered_instance_ids,
        instance_lock_mgr,
    })
}
