use crate::group_persistence_command_like_cpp;
use wow_core::ObjectGuid;
use wow_social::group::GroupInfo;

#[test]
fn group_insert_intent_maps_to_sqlx_free_command_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let group = GroupInfo::new(leader);
    let command = group_persistence_command_like_cpp(
        wow_social::group::GroupPersistenceIntentLikeCpp::InsertGroup {
            db_store_id: 77,
            leader_guid: group.leader_guid,
            loot_method: group.loot_method,
            looter_guid: group.looter_guid,
            loot_threshold: group.loot_threshold,
            group_flags: group.group_flags,
            dungeon_difficulty_id: group.dungeon_difficulty_id,
            raid_difficulty_id: group.raid_difficulty_id,
            legacy_raid_difficulty_id: group.legacy_raid_difficulty_id,
            master_looter_guid: group.master_looter_guid,
        },
    );
    assert_eq!(
        command,
        wow_persistence::RepresentedGroupPersistenceCommandLikeCpp::InsertGroup {
            db_store_id: 77,
            leader_guid: leader.counter() as u64,
            loot_method: wow_social::group::LOOT_METHOD_PERSONAL_LIKE_CPP,
            looter_guid: leader.counter() as u64,
            loot_threshold: 2,
            group_flags: 0,
            dungeon_difficulty_id: 1,
            raid_difficulty_id: 14,
            legacy_raid_difficulty_id: 3,
            master_looter_guid: 0,
        }
    );
}

#[test]
fn group_leave_intents_map_to_order_preserving_sqlx_free_commands_like_cpp() {
    let old_member = ObjectGuid::create_player(1, 42);
    let new_leader = ObjectGuid::create_player(1, 77);
    assert_eq!(
        [
            group_persistence_command_like_cpp(
                wow_social::group::GroupPersistenceIntentLikeCpp::DeleteMember {
                    member_guid: old_member,
                }
            ),
            group_persistence_command_like_cpp(
                wow_social::group::GroupPersistenceIntentLikeCpp::UpdateLeader {
                    db_store_id: 99,
                    leader_guid: new_leader,
                }
            ),
            group_persistence_command_like_cpp(
                wow_social::group::GroupPersistenceIntentLikeCpp::DeleteGroup { db_store_id: 99 }
            ),
        ],
        [
            wow_persistence::RepresentedGroupPersistenceCommandLikeCpp::DeleteMember {
                member_guid: old_member.counter() as u64,
            },
            wow_persistence::RepresentedGroupPersistenceCommandLikeCpp::UpdateLeader {
                db_store_id: 99,
                leader_guid: new_leader.counter() as u64,
            },
            wow_persistence::RepresentedGroupPersistenceCommandLikeCpp::DeleteGroup {
                db_store_id: 99,
            },
        ]
    );
}
