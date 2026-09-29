//! Fixture surface prepared for the crate's integration tests.
//!
//! Issue #584 (B3): these re-exports and wrappers let external integration-test targets
//! reach crate-level fixtures. The suite migration is still pending; this module is
//! available only in the `test-fixtures` build.

mod quest;
mod player;
pub use quest::*;
pub use player::*;

pub use crate::session::quest_dialog::{
    RepresentedPendingQuestSharingLikeCpp,
    RepresentedAdventureMapStartQuestLikeCpp,
    RepresentedPushQuestToPartyOutcomeLikeCpp,
    RepresentedPushQuestToPartyOutcomeReasonLikeCpp,
    RepresentedQuestConfirmAcceptLikeCpp,
    RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp,
    RepresentedQuestPushResultResponseLikeCpp,
    RepresentedQuestRewardReputationLikeCpp,
    RepresentedQuestRewardReputationSourceLikeCpp,
    RepresentedQuestRewardSpellCastLikeCpp,
    RepresentedQuestRewardSpellKindLikeCpp,
    RepresentedQuestRewardTitleLikeCpp,
    RepresentedQuestRewardTalentPointsLikeCpp,
    RepresentedQuestRewardMailLikeCpp,
};

pub const QUEST_FLAGS_TRACKING_EVENT_LIKE_CPP: u32 =
    crate::handlers::quest::QUEST_FLAGS_TRACKING_EVENT_LIKE_CPP;
pub const QUEST_OBJECTIVE_CURRENCY_LIKE_CPP_LOCAL: u8 =
    crate::handlers::quest::QUEST_OBJECTIVE_CURRENCY_LIKE_CPP_LOCAL;
pub const QUEST_OBJECTIVE_MONEY_LIKE_CPP_LOCAL: u8 =
    crate::handlers::quest::QUEST_OBJECTIVE_MONEY_LIKE_CPP_LOCAL;
pub const QUEST_FLAGS_PLAYER_CAST_COMPLETE_LIKE_CPP: u32 =
    crate::handlers::quest::QUEST_FLAGS_PLAYER_CAST_COMPLETE_LIKE_CPP;
pub const CURRENCY_DESTROY_REASON_QUEST_TURNIN_LIKE_CPP: i32 =
    crate::handlers::quest::CURRENCY_DESTROY_REASON_QUEST_TURNIN_LIKE_CPP;

pub fn represented_quest_reward_spell_casts_for_test(
    session: &crate::session::WorldSession,
) -> Vec<RepresentedQuestRewardSpellCastLikeCpp> {
    session
        .represented_quest_reward_spell_casts_like_cpp()
        .to_vec()
}

pub fn represented_quest_reward_titles_for_test(
    session: &crate::session::WorldSession,
) -> Vec<RepresentedQuestRewardTitleLikeCpp> {
    session.represented_quest_reward_titles_like_cpp().to_vec()
}

pub fn represented_quest_reward_talent_points_for_test(
    session: &crate::session::WorldSession,
) -> Vec<RepresentedQuestRewardTalentPointsLikeCpp> {
    session
        .represented_quest_reward_talent_points_like_cpp()
        .to_vec()
}

pub fn represented_quest_reward_mails_for_test(
    session: &crate::session::WorldSession,
) -> Vec<RepresentedQuestRewardMailLikeCpp> {
    session.represented_quest_reward_mails_like_cpp().to_vec()
}

pub fn represented_adventure_map_start_quest_requests_for_test(
    session: &crate::session::WorldSession,
) -> Vec<RepresentedAdventureMapStartQuestLikeCpp> {
    session
        .represented_adventure_map_start_quest_requests_like_cpp()
        .to_vec()
}

pub fn represented_quest_push_result_responses_for_test(
    session: &crate::session::WorldSession,
) -> Vec<RepresentedQuestPushResultResponseLikeCpp> {
    session
        .represented_quest_push_result_responses_like_cpp()
        .to_vec()
}

pub fn represented_quest_push_result_sender_mismatch_count_for_test(
    session: &crate::session::WorldSession,
) -> u32 {
    session.represented_quest_push_result_sender_mismatch_count_like_cpp()
}

pub fn read_quest_choice_item_for_test(
    pkt: &mut wow_packet::WorldPacket,
) -> Result<(u8, u32, i32), wow_packet::PacketError> {
    crate::handlers::quest::read_quest_choice_item_tuple_for_test(pkt)
}

pub fn represented_reward_choice_matches_loaded_type_for_test(
    quest: &wow_data::quest::QuestTemplate,
    loot_item_type: u8,
    item_id: u32,
    quantity: i32,
) -> bool {
    crate::handlers::quest::represented_reward_choice_matches_loaded_type_tuple_for_test(
        quest,
        loot_item_type,
        item_id,
        quantity,
    )
}

pub async fn quest_poi_store_for_test(
    session: &mut crate::session::WorldSession,
) -> std::sync::Arc<
    std::collections::HashMap<i32, wow_packet::packets::query::QuestPoiData>,
> {
    crate::handlers::quest::quest_poi_store_for_test(session).await
}

pub fn set_quest_poi_store_for_test(
    session: &mut crate::session::WorldSession,
    store: std::sync::Arc<
        std::collections::HashMap<i32, wow_packet::packets::query::QuestPoiData>,
    >,
) {
    session.quest_poi_store_like_cpp = Some(store);
}

pub fn quest_log_create_entries_for_test(
    session: &crate::session::WorldSession,
) -> Vec<(u32, u32, i64, [u16; 24])> {
    session.quest_log_create_entries_like_cpp()
}

pub use crate::handlers::group::state::PARTY_REALM_COMMAND_TIMEOUT_LIKE_CPP;
pub use crate::handlers::group::state::current_group_guid_like_cpp;
pub use crate::handlers::group::state::first_connected_group_member_like_cpp;
pub use crate::handlers::group::state::group_persistence_command_like_cpp;
pub use crate::handlers::group::state::party_player_info_like_cpp;
pub use crate::handlers::group::state::send_group_new_leader_like_cpp;
pub use crate::handlers::group::state::send_party_update;
pub use crate::handlers::group::state::send_ready_check_events_like_cpp;
pub use crate::handlers::group::state::sender_can_start_ready_check_like_cpp;
pub use crate::handlers::group::test_support::{
    PartyInviteSocialPortLikeCpp, persist_group_intents_like_cpp,
};

pub async fn handle_party_invite_with_policy_like_cpp(
    session: &mut crate::session::WorldSession,
    pkt: wow_packet::WorldPacket,
    policy: &crate::session::GroupInvitePolicyLikeCpp,
) {
    session
        .handle_party_invite_with_policy_like_cpp(pkt, policy)
        .await;
}

pub fn set_group_guid_for_test_like_cpp(
    session: &mut crate::session::WorldSession,
    group_guid: Option<u64>,
) {
    session.group_guid = group_guid;
}

pub fn group_guid_for_test_like_cpp(
    session: &crate::session::WorldSession,
) -> Option<u64> {
    session.group_guid
}

pub fn set_pass_on_group_loot_for_test_like_cpp(
    session: &mut crate::session::WorldSession,
    pass_on_group_loot: bool,
) {
    session.pass_on_group_loot = pass_on_group_loot;
}

pub fn pass_on_group_loot_for_test_like_cpp(
    session: &crate::session::WorldSession,
) -> bool {
    session.pass_on_group_loot
}

pub fn set_in_combat_for_test_like_cpp(
    session: &mut crate::session::WorldSession,
    in_combat: bool,
) {
    session.in_combat = in_combat;
}

pub fn in_combat_for_test_like_cpp(session: &crate::session::WorldSession) -> bool {
    session.in_combat
}

pub fn with_canonical_player_at_mut_like_cpp<R>(
    manager: &crate::session::SharedCanonicalMapManager,
    guid: wow_core::ObjectGuid,
    map_id: u32,
    instance_id: u32,
    mutate: impl FnOnce(&mut wow_entities::Player) -> R,
) -> Option<R> {
    crate::canonical_player_access::with_canonical_player_at_mut_like_cpp(
        manager,
        guid,
        map_id,
        instance_id,
        mutate,
    )
}

pub fn canonical_map_manager_for_test(
    session: &crate::session::WorldSession,
) -> Option<&crate::session::SharedCanonicalMapManager> {
    session.canonical_map_manager.as_ref()
}

pub fn set_loaded_player_identity_like_cpp(
    session: &mut crate::session::WorldSession,
    map_id: u16,
    race: u8,
    class: u8,
    level: u8,
    gender: u8,
) {
    session.set_loaded_player_identity_like_cpp(map_id, race, class, level, gender);
}

pub fn set_loaded_player_name_like_cpp(
    session: &mut crate::session::WorldSession,
    name: String,
) {
    session.set_loaded_player_name_like_cpp(name);
}

pub fn set_player_gold_for_test(
    session: &mut crate::session::WorldSession,
    gold: u64,
) -> bool {
    session.set_player_gold_like_cpp(gold)
}

pub fn player_gold_for_test(session: &crate::session::WorldSession) -> u64 {
    session.player_gold_like_cpp()
}

pub fn set_loot_money_persistence_test_result_for_test(
    session: &mut crate::session::WorldSession,
    success: bool,
) {
    session.set_loot_money_persistence_test_result_like_cpp(success);
}

pub fn inventory_items_for_test(
    session: &crate::session::WorldSession,
) -> &std::collections::HashMap<u8, crate::session::InventoryItem> {
    session.inventory_items_like_cpp()
}

pub fn inventory_item_objects_for_test(
    session: &crate::session::WorldSession,
) -> &std::collections::HashMap<wow_core::ObjectGuid, wow_entities::Item> {
    session.inventory_item_objects_like_cpp()
}

pub fn make_inventory_item_object_for_test(
    session: &crate::session::WorldSession,
    item_guid: wow_core::ObjectGuid,
    entry_id: u32,
    owner_guid: wow_core::ObjectGuid,
    count: u32,
    durability: u32,
    context: wow_constants::ItemContext,
    slot: u8,
) -> wow_entities::Item {
    session.make_inventory_item_object(
        item_guid,
        entry_id,
        owner_guid,
        count,
        durability,
        context,
        slot,
    )
}

pub fn insert_inventory_item_object_for_test(
    session: &mut crate::session::WorldSession,
    item: wow_entities::Item,
) -> Option<wow_entities::Item> {
    session.insert_inventory_item_object(item)
}

pub fn insert_inventory_item_for_test(
    session: &mut crate::session::WorldSession,
    slot: u8,
    item: crate::session::InventoryItem,
) -> Option<crate::session::InventoryItem> {
    session.insert_inventory_item_like_cpp(slot, item)
}

pub fn drain_session_commands_for_test(
    session: &crate::session::WorldSession,
) -> Vec<crate::session::mailbox::SessionCommand> {
    session.drain_session_commands()
}

pub async fn quest_source_item_quest_log_item_id_for_test(
    session: &mut crate::session::WorldSession,
    entry_id: u32,
) -> u32 {
    session
        .quest_source_item_quest_log_item_id_like_cpp(entry_id)
        .await
}

pub fn cache_item_template_addon_quest_log_item_id_for_test(
    session: &mut crate::session::WorldSession,
    item_id: u32,
    quest_log_item_id: u32,
) {
    session.cache_item_template_addon_quest_log_item_id_like_cpp(item_id, quest_log_item_id);
}

pub fn set_represented_daily_quest_completed_for_test(
    session: &mut crate::session::WorldSession,
    quest_id: u32,
    completed: bool,
) {
    session.set_represented_daily_quest_completed_like_cpp_for_test(quest_id, completed);
}

pub fn adopt_registered_canonical_player_fixture_like_cpp(
    session: &mut crate::session::WorldSession,
) -> bool {
    session.adopt_registered_canonical_player_fixture_like_cpp()
}

pub fn sync_player_registry_state_for_test(session: &crate::session::WorldSession) {
    session.sync_player_registry_state_like_cpp();
}

pub fn register_in_player_registry_for_test(session: &crate::session::WorldSession) {
    session.register_in_player_registry();
}

pub fn set_represented_dungeon_difficulty_id_for_test_like_cpp(
    session: &mut crate::session::WorldSession,
    difficulty_id: u32,
) {
    session.set_represented_dungeon_difficulty_id_for_test_like_cpp(difficulty_id);
}

pub fn resolved_dungeon_difficulty_id_like_cpp(
    session: &crate::session::WorldSession,
) -> Option<u32> {
    session.resolved_dungeon_difficulty_id_like_cpp()
}

pub fn set_player_battleground_type_id_like_cpp(
    session: &mut crate::session::WorldSession,
    bg_type_id: u32,
) -> bool {
    session.set_player_battleground_type_id_like_cpp(bg_type_id)
}

pub fn represented_subgroup_like_cpp(
    session: &crate::session::WorldSession,
) -> Option<u8> {
    session.represented_subgroup_like_cpp()
}

pub fn install_realm_send_channel_for_test(
    session: &mut crate::session::WorldSession,
    tx: flume::Sender<Vec<u8>>,
) {
    session.install_realm_send_channel_for_test(tx);
}

pub fn set_in_combat_like_cpp(session: &mut crate::session::WorldSession, in_combat: bool) {
    session.set_in_combat_like_cpp(in_combat);
}

pub fn set_owned_player_group_like_cpp(
    session: &mut crate::session::WorldSession,
    membership: Option<(u64, u8)>,
) -> bool {
    session.set_owned_player_group_like_cpp(membership)
}

pub fn resolved_group_guid_like_cpp(
    session: &crate::session::WorldSession,
) -> Option<u64> {
    session.resolved_group_guid_like_cpp()
}

pub fn reconcile_group_state_like_cpp(session: &mut crate::session::WorldSession) -> bool {
    session.reconcile_group_state_like_cpp()
}

pub fn represented_silence_party_talker_like_cpp(
    session: &crate::session::WorldSession,
) -> Vec<(wow_core::ObjectGuid, bool)> {
    session
        .represented_silence_party_talker_like_cpp()
        .iter()
        .map(|record| (record.target.clone(), record.silent))
        .collect()
}

pub async fn process_represented_session_commands_like_cpp(
    session: &mut crate::session::WorldSession,
) {
    session
        .process_represented_session_commands_like_cpp()
        .await;
}

pub fn insert_player_quest_status_for_test(
    session: &mut crate::session::WorldSession,
    quest_id: u32,
    status: crate::handlers::quest::PlayerQuestStatus,
) -> Option<crate::handlers::quest::PlayerQuestStatus> {
    session
        .quest_test_fixture_like_cpp
        .player_quests
        .insert(quest_id, status)
}

pub fn contains_player_quest_status_for_test(
    session: &crate::session::WorldSession,
    quest_id: u32,
) -> bool {
    session
        .quest_test_fixture_like_cpp
        .player_quests
        .contains_key(&quest_id)
}

pub fn player_quest_status_for_test(
    session: &crate::session::WorldSession,
    quest_id: u32,
) -> Option<crate::handlers::quest::PlayerQuestStatus> {
    session
        .quest_test_fixture_like_cpp
        .player_quests
        .get(&quest_id)
        .cloned()
}

pub fn with_player_quest_status_mut_for_test<R>(
    session: &mut crate::session::WorldSession,
    id: u32,
    f: impl FnOnce(&mut crate::handlers::quest::PlayerQuestStatus) -> R,
) -> Option<R> {
    session
        .quest_test_fixture_like_cpp
        .player_quests
        .get_mut(&id)
        .map(f)
}

pub fn player_quest_statuses_for_test(
    session: &crate::session::WorldSession,
) -> Vec<crate::handlers::quest::PlayerQuestStatus> {
    session
        .quest_test_fixture_like_cpp
        .player_quests
        .values()
        .cloned()
        .collect()
}

pub fn insert_rewarded_quest_for_test(
    session: &mut crate::session::WorldSession,
    id: u32,
) -> bool {
    session
        .quest_test_fixture_like_cpp
        .rewarded_quests
        .insert(id)
}

pub fn contains_rewarded_quest_for_test(
    session: &crate::session::WorldSession,
    id: u32,
) -> bool {
    session
        .quest_test_fixture_like_cpp
        .rewarded_quests
        .contains(&id)
}

pub fn get_quest_slot_quest_id_for_test(
    session: &crate::session::WorldSession,
    slot: u8,
) -> Option<u32> {
    session.get_quest_slot_quest_id_like_cpp(slot)
}

pub fn set_player_level_for_test(session: &mut crate::session::WorldSession, level: u8) {
    session.set_player_level_like_cpp(level);
}

pub fn insert_daily_quest_completed_for_test(
    session: &mut crate::session::WorldSession,
    id: u32,
) -> bool {
    session
        .quest_test_fixture_like_cpp
        .daily_quests_completed_like_cpp
        .insert(id)
}

pub fn contains_daily_quest_completed_for_test(
    session: &crate::session::WorldSession,
    id: u32,
) -> bool {
    session
        .quest_test_fixture_like_cpp
        .daily_quests_completed_like_cpp
        .contains(&id)
}

pub fn insert_df_quest_for_test(
    session: &mut crate::session::WorldSession,
    id: u32,
) -> bool {
    session
        .quest_test_fixture_like_cpp
        .df_quests_like_cpp
        .insert(id)
}

pub fn contains_df_quest_for_test(
    session: &crate::session::WorldSession,
    id: u32,
) -> bool {
    session
        .quest_test_fixture_like_cpp
        .df_quests_like_cpp
        .contains(&id)
}

pub fn insert_weekly_quest_completed_for_test(
    session: &mut crate::session::WorldSession,
    id: u32,
) -> bool {
    session
        .quest_test_fixture_like_cpp
        .weekly_quests_completed_like_cpp
        .insert(id)
}

pub fn contains_weekly_quest_completed_for_test(
    session: &crate::session::WorldSession,
    id: u32,
) -> bool {
    session
        .quest_test_fixture_like_cpp
        .weekly_quests_completed_like_cpp
        .contains(&id)
}

pub fn insert_monthly_quest_completed_for_test(
    session: &mut crate::session::WorldSession,
    id: u32,
) -> bool {
    session
        .quest_test_fixture_like_cpp
        .monthly_quests_completed_like_cpp
        .insert(id)
}

pub fn contains_monthly_quest_completed_for_test(
    session: &crate::session::WorldSession,
    id: u32,
) -> bool {
    session
        .quest_test_fixture_like_cpp
        .monthly_quests_completed_like_cpp
        .contains(&id)
}

pub fn last_daily_quest_time_for_test(session: &crate::session::WorldSession) -> i64 {
    session
        .quest_test_fixture_like_cpp
        .last_daily_quest_time_like_cpp
}

pub fn contains_seasonal_quest_for_test(
    session: &crate::session::WorldSession,
    event_id: u16,
    quest_id: u32,
) -> bool {
    session
        .quest_test_fixture_like_cpp
        .seasonal_quests_like_cpp
        .get(&event_id)
        .is_some_and(|quests| quests.contains_key(&quest_id))
}

pub fn seasonal_quest_changed_for_test(session: &crate::session::WorldSession) -> bool {
    session
        .quest_test_fixture_like_cpp
        .seasonal_quest_changed_like_cpp
}

pub fn represented_auto_accept_acknowledged_quests_for_test(
    session: &crate::session::WorldSession,
) -> Vec<u32> {
    session
        .quest_test_fixture_like_cpp
        .represented_auto_accept_acknowledged_quests_like_cpp
        .clone()
}

pub fn represented_push_quest_to_party_outcomes_for_test(
    session: &crate::session::WorldSession,
) -> Vec<RepresentedPushQuestToPartyOutcomeLikeCpp> {
    session
        .represented_push_quest_to_party_outcomes_like_cpp()
        .to_vec()
}

pub fn represented_quest_confirm_accepts_for_test(
    session: &crate::session::WorldSession,
) -> Vec<RepresentedQuestConfirmAcceptLikeCpp> {
    session.represented_quest_confirm_accepts_like_cpp().to_vec()
}

pub fn represented_quest_reward_reputations_for_test(
    session: &crate::session::WorldSession,
) -> Vec<RepresentedQuestRewardReputationLikeCpp> {
    session
        .represented_quest_reward_reputations_like_cpp()
        .to_vec()
}

pub fn set_represented_pending_quest_sharing_for_test(
    session: &mut crate::session::WorldSession,
    sender_guid: wow_core::ObjectGuid,
    quest_id: u32,
) {
    session.set_represented_pending_quest_sharing_like_cpp(sender_guid, quest_id);
}

pub fn represented_pending_quest_sharing_for_test(
    session: &crate::session::WorldSession,
) -> Option<RepresentedPendingQuestSharingLikeCpp> {
    session.represented_pending_quest_sharing_like_cpp()
}

pub const PLAYER_FLAGS_GHOST_LIKE_CPP: u32 = crate::session::PLAYER_FLAGS_GHOST_LIKE_CPP;
pub const PLAYER_FLAGS_AFK_LIKE_CPP: u32 = crate::session::PLAYER_FLAGS_AFK_LIKE_CPP;
pub const PLAYER_FLAGS_DND_LIKE_CPP: u32 = crate::session::PLAYER_FLAGS_DND_LIKE_CPP;

pub const QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP: u32 =
    crate::handlers::quest::QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP;
pub const QUEST_FLAGS_SHARABLE_LIKE_CPP: u32 = crate::handlers::quest::QUEST_FLAGS_SHARABLE_LIKE_CPP;
pub const QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP: u8 =
    crate::handlers::quest::QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP;
pub const QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP: u8 =
    crate::handlers::quest::QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP;
pub const QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL: u8 =
    wow_constants::quest::QUEST_OBJECTIVE_ITEM_LIKE_CPP;
pub const QUEST_OBJECTIVE_MONSTER_LIKE_CPP_LOCAL: u8 =
    wow_constants::quest::QUEST_OBJECTIVE_MONSTER_LIKE_CPP;
pub const QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP_LOCAL: u32 =
    wow_constants::quest::QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP;
pub const MAX_QUEST_LOG_SIZE_LIKE_CPP: u8 = crate::handlers::quest::MAX_QUEST_LOG_SIZE_LIKE_CPP;
pub const QUEST_PUSH_REASON_INVALID_LIKE_CPP: u8 =
    crate::handlers::quest::QUEST_PUSH_REASON_INVALID_LIKE_CPP;
pub const QUEST_PUSH_REASON_INVALID_TO_RECIPIENT_LIKE_CPP: u8 =
    crate::handlers::quest::QUEST_PUSH_REASON_INVALID_TO_RECIPIENT_LIKE_CPP;
pub const QUEST_PUSH_REASON_BUSY_LIKE_CPP: u8 =
    crate::handlers::quest::QUEST_PUSH_REASON_BUSY_LIKE_CPP;
pub const QUEST_PUSH_REASON_DEAD_LIKE_CPP: u8 =
    crate::handlers::quest::QUEST_PUSH_REASON_DEAD_LIKE_CPP;
pub const QUEST_PUSH_REASON_DEAD_TO_RECIPIENT_LIKE_CPP: u8 =
    crate::handlers::quest::QUEST_PUSH_REASON_DEAD_TO_RECIPIENT_LIKE_CPP;
pub const QUEST_PUSH_REASON_LOG_FULL_LIKE_CPP: u8 =
    crate::handlers::quest::QUEST_PUSH_REASON_LOG_FULL_LIKE_CPP;
pub const QUEST_PUSH_REASON_LOG_FULL_TO_RECIPIENT_LIKE_CPP: u8 =
    crate::handlers::quest::QUEST_PUSH_REASON_LOG_FULL_TO_RECIPIENT_LIKE_CPP;
pub const QUEST_PUSH_REASON_ON_QUEST_LIKE_CPP: u8 =
    crate::handlers::quest::QUEST_PUSH_REASON_ON_QUEST_LIKE_CPP;
pub const QUEST_PUSH_REASON_ON_QUEST_TO_RECIPIENT_LIKE_CPP: u8 =
    crate::handlers::quest::QUEST_PUSH_REASON_ON_QUEST_TO_RECIPIENT_LIKE_CPP;
pub const QUEST_PUSH_REASON_ALREADY_DONE_LIKE_CPP: u8 =
    crate::handlers::quest::QUEST_PUSH_REASON_ALREADY_DONE_LIKE_CPP;
pub const QUEST_PUSH_REASON_ALREADY_DONE_TO_RECIPIENT_LIKE_CPP: u8 =
    crate::handlers::quest::QUEST_PUSH_REASON_ALREADY_DONE_TO_RECIPIENT_LIKE_CPP;
pub const QUEST_PUSH_REASON_PREREQUISITE_LIKE_CPP: u8 =
    crate::handlers::quest::QUEST_PUSH_REASON_PREREQUISITE_LIKE_CPP;
pub const QUEST_PUSH_REASON_PREREQUISITE_TO_RECIPIENT_LIKE_CPP: u8 =
    crate::handlers::quest::QUEST_PUSH_REASON_PREREQUISITE_TO_RECIPIENT_LIKE_CPP;
pub const QUEST_PUSH_REASON_LOW_LEVEL_LIKE_CPP: u8 =
    crate::handlers::quest::QUEST_PUSH_REASON_LOW_LEVEL_LIKE_CPP;
pub const QUEST_PUSH_REASON_LOW_LEVEL_TO_RECIPIENT_LIKE_CPP: u8 =
    crate::handlers::quest::QUEST_PUSH_REASON_LOW_LEVEL_TO_RECIPIENT_LIKE_CPP;
pub const QUEST_PUSH_REASON_HIGH_LEVEL_LIKE_CPP: u8 =
    crate::handlers::quest::QUEST_PUSH_REASON_HIGH_LEVEL_LIKE_CPP;
pub const QUEST_PUSH_REASON_HIGH_LEVEL_TO_RECIPIENT_LIKE_CPP: u8 =
    crate::handlers::quest::QUEST_PUSH_REASON_HIGH_LEVEL_TO_RECIPIENT_LIKE_CPP;
pub const QUEST_PUSH_REASON_CLASS_LIKE_CPP: u8 =
    crate::handlers::quest::QUEST_PUSH_REASON_CLASS_LIKE_CPP;
pub const QUEST_PUSH_REASON_CLASS_TO_RECIPIENT_LIKE_CPP: u8 =
    crate::handlers::quest::QUEST_PUSH_REASON_CLASS_TO_RECIPIENT_LIKE_CPP;
pub const QUEST_PUSH_REASON_RACE_LIKE_CPP: u8 =
    crate::handlers::quest::QUEST_PUSH_REASON_RACE_LIKE_CPP;
pub const QUEST_PUSH_REASON_RACE_TO_RECIPIENT_LIKE_CPP: u8 =
    crate::handlers::quest::QUEST_PUSH_REASON_RACE_TO_RECIPIENT_LIKE_CPP;
pub const QUEST_PUSH_REASON_LOW_FACTION_LIKE_CPP: u8 =
    crate::handlers::quest::QUEST_PUSH_REASON_LOW_FACTION_LIKE_CPP;
pub const QUEST_PUSH_REASON_LOW_FACTION_TO_RECIPIENT_LIKE_CPP: u8 =
    crate::handlers::quest::QUEST_PUSH_REASON_LOW_FACTION_TO_RECIPIENT_LIKE_CPP;
pub const QUEST_PUSH_REASON_EXPANSION_LIKE_CPP: u8 =
    crate::handlers::quest::QUEST_PUSH_REASON_EXPANSION_LIKE_CPP;
pub const QUEST_PUSH_REASON_EXPANSION_TO_RECIPIENT_LIKE_CPP: u8 =
    crate::handlers::quest::QUEST_PUSH_REASON_EXPANSION_TO_RECIPIENT_LIKE_CPP;
pub const QUEST_PUSH_REASON_SUCCESS_LIKE_CPP: u8 =
    crate::handlers::quest::QUEST_PUSH_REASON_SUCCESS_LIKE_CPP;
