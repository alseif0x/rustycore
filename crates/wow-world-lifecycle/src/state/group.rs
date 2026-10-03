use tracing::warn;
use wow_persistence::{
    RepresentedGroupDifficultyKindLikeCpp, RepresentedGroupPersistenceCommandLikeCpp,
    RepresentedGroupPersistenceModeLikeCpp, RepresentedGroupPersistenceOutcomeLikeCpp,
    RepresentedGroupPersistenceRequestLikeCpp,
};
use wow_social::group::{GroupDifficultyKindLikeCpp, GroupPersistenceIntentLikeCpp};

use super::SessionLifecycleState;

impl SessionLifecycleState {
    pub async fn persist_group_intents_like_cpp(
        &self,
        group_guid: u64,
        intents: Vec<GroupPersistenceIntentLikeCpp>,
    ) {
        let Some(port) = self.represented_group_persistence_port_like_cpp() else {
            return;
        };
        let request = RepresentedGroupPersistenceRequestLikeCpp {
            commands: intents
                .into_iter()
                .map(group_persistence_command_like_cpp)
                .collect(),
            mode: RepresentedGroupPersistenceModeLikeCpp::Sequential,
        };
        let outcome = port.persist_group_commands_like_cpp(request).await;
        if !matches!(
            outcome,
            RepresentedGroupPersistenceOutcomeLikeCpp::Applied { .. }
        ) {
            warn!(
                group_guid,
                ?outcome,
                "failed to persist represented group transition"
            );
        }
    }
}

/// Application mapping from the database-neutral commands emitted by
/// `GroupRegistry` to the SQLx-free persistence vocabulary. The vector order
/// remains the order selected by the aggregate and no registry guard survives
/// into adapter execution.
pub fn group_persistence_command_like_cpp(
    intent: GroupPersistenceIntentLikeCpp,
) -> RepresentedGroupPersistenceCommandLikeCpp {
    match intent {
        GroupPersistenceIntentLikeCpp::InsertGroup {
            db_store_id,
            leader_guid,
            loot_method,
            looter_guid,
            loot_threshold,
            group_flags,
            dungeon_difficulty_id,
            raid_difficulty_id,
            legacy_raid_difficulty_id,
            master_looter_guid,
        } => RepresentedGroupPersistenceCommandLikeCpp::InsertGroup {
            db_store_id,
            leader_guid: leader_guid.counter() as u64,
            loot_method,
            looter_guid: looter_guid.counter() as u64,
            loot_threshold,
            group_flags,
            dungeon_difficulty_id,
            raid_difficulty_id,
            legacy_raid_difficulty_id,
            master_looter_guid: master_looter_guid.counter() as u64,
        },
        GroupPersistenceIntentLikeCpp::InsertMember {
            db_store_id,
            member_guid,
            member_flags,
            subgroup,
            roles,
        } => RepresentedGroupPersistenceCommandLikeCpp::InsertMember {
            db_store_id,
            member_guid: member_guid.counter() as u64,
            member_flags,
            subgroup,
            roles,
        },
        GroupPersistenceIntentLikeCpp::DeleteGroup { db_store_id } => {
            RepresentedGroupPersistenceCommandLikeCpp::DeleteGroup { db_store_id }
        }
        GroupPersistenceIntentLikeCpp::DeleteAllMembers { db_store_id } => {
            RepresentedGroupPersistenceCommandLikeCpp::DeleteAllMembers { db_store_id }
        }
        GroupPersistenceIntentLikeCpp::DeleteLfgData { db_store_id } => {
            RepresentedGroupPersistenceCommandLikeCpp::DeleteLfgData { db_store_id }
        }
        GroupPersistenceIntentLikeCpp::DeleteMember { member_guid } => {
            RepresentedGroupPersistenceCommandLikeCpp::DeleteMember {
                member_guid: member_guid.counter() as u64,
            }
        }
        GroupPersistenceIntentLikeCpp::UpdateLeader {
            db_store_id,
            leader_guid,
        } => RepresentedGroupPersistenceCommandLikeCpp::UpdateLeader {
            db_store_id,
            leader_guid: leader_guid.counter() as u64,
        },
        GroupPersistenceIntentLikeCpp::UpdateGroupType {
            db_store_id,
            group_flags,
        } => RepresentedGroupPersistenceCommandLikeCpp::UpdateGroupType {
            db_store_id,
            group_flags,
        },
        GroupPersistenceIntentLikeCpp::UpdateMemberSubgroup {
            member_guid,
            subgroup,
        } => RepresentedGroupPersistenceCommandLikeCpp::UpdateMemberSubgroup {
            member_guid: member_guid.counter() as u64,
            subgroup,
        },
        GroupPersistenceIntentLikeCpp::UpdateMemberFlags { member_guid, flags } => {
            RepresentedGroupPersistenceCommandLikeCpp::UpdateMemberFlags {
                member_guid: member_guid.counter() as u64,
                flags,
            }
        }
        GroupPersistenceIntentLikeCpp::UpdateDifficulty {
            db_store_id,
            kind,
            difficulty_id,
        } => RepresentedGroupPersistenceCommandLikeCpp::UpdateDifficulty {
            db_store_id,
            kind: match kind {
                GroupDifficultyKindLikeCpp::Dungeon => {
                    RepresentedGroupDifficultyKindLikeCpp::Dungeon
                }
                GroupDifficultyKindLikeCpp::Raid => {
                    RepresentedGroupDifficultyKindLikeCpp::Raid
                }
                GroupDifficultyKindLikeCpp::LegacyRaid => {
                    RepresentedGroupDifficultyKindLikeCpp::LegacyRaid
                }
            },
            difficulty_id,
        },
    }
}

#[cfg(test)]
#[path = "../../unit_tests/group_mapping.rs"]
mod tests;
