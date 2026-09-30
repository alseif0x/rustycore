//! Shared group registry and represented persistence startup.

use std::sync::Arc;

use anyhow::Context;
use tracing::info;
use wow_social::group::{GroupLoadSummaryLikeCpp, GroupRegistry, PendingInvites};

use crate::runtime::load_groups_from_character_database_like_cpp;

pub(super) struct GroupStartup {
    pub(super) group_registry: Arc<GroupRegistry>,
    pub(super) pending_invites: Arc<PendingInvites>,
    pub(super) represented_group_persistence_adapter: Arc<wow_database::represented_group_persistence_adapter::MariaDbRepresentedGroupPersistenceAdapterLikeCpp>,
    pub(super) group_load_summary: GroupLoadSummaryLikeCpp,
}

pub(super) async fn load_group_startup(
    char_db: &Arc<wow_database::CharacterDatabase>,
    difficulty_store: &wow_data::DifficultyStore,
) -> anyhow::Result<GroupStartup> {
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

    Ok(GroupStartup {
        group_registry,
        pending_invites,
        represented_group_persistence_adapter,
        group_load_summary,
    })
}
