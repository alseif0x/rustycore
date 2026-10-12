//! Responsibility-scoped builders shared by the Session scenario modules.
//!
//! The parent `session::tests` module keeps the scenario registration root and
//! re-exports this narrow fixture surface so existing `use super::*` consumers
//! retain their private access without a second test-support aggregate.

use super::*;

#[path = "canonical_creatures.rs"]
mod canonical_creature_fixtures;
#[path = "creature_ai.rs"]
mod creature_ai_fixtures;
#[path = "creatures.rs"]
mod creature_fixtures;
#[path = "instances.rs"]
mod instance_fixtures;
#[path = "interaction.rs"]
mod interaction_fixtures;
#[path = "item_collections.rs"]
mod item_collection_fixtures;
#[path = "lifecycle.rs"]
mod lifecycle_fixtures;
#[path = "login.rs"]
mod login_fixtures;
#[path = "map_objects.rs"]
mod map_object_fixtures;
#[path = "packets.rs"]
mod packet_fixtures;
#[path = "player_items.rs"]
mod player_item_fixtures;
#[path = "quests.rs"]
mod quest_fixtures;
#[path = "session.rs"]
mod session_fixtures;
#[path = "spell_catalog.rs"]
mod spell_catalog_fixtures;
#[path = "summons.rs"]
mod summon_fixtures;
#[path = "visibility.rs"]
mod visibility_fixtures;

pub(super) use crate::canonical_creature_fixtures::*;
// #1363 slice 3b: the session-free single-phase drivers moved next to the
// runtime they drive.
pub(super) use canonical_creature_fixtures::*;
pub(super) use creature_ai_fixtures::*;
pub(super) use creature_fixtures::*;
pub(super) use instance_fixtures::*;
pub(super) use interaction_fixtures::*;
pub(super) use item_collection_fixtures::*;
pub(super) use lifecycle_fixtures::*;
pub(super) use login_fixtures::*;
pub(super) use map_object_fixtures::*;
pub(super) use packet_fixtures::*;
pub(super) use player_item_fixtures::*;
pub(super) use quest_fixtures::*;
pub(super) use session_fixtures::*;
pub(super) use spell_catalog_fixtures::*;
pub(super) use summon_fixtures::*;
pub(super) use visibility_fixtures::*;
pub(super) use wow_world_creature::canonical_creatures::*;
// #1363 slice 0: the session-free builders of these modules live in the dev-only
// `wow-world-test-support` crate; the scenario modules keep reaching them here.
pub(super) use wow_world_test_support::creature_ai::{
    CreatureSpellWireHeaderLikeCpp, add_canonical_creature_spell_test_pair_like_cpp,
    add_canonical_creature_spell_test_pair_with_difficulty_like_cpp, aggro_candidate_like_cpp,
    aggro_hostile_config_like_cpp, aggro_relation_config_like_cpp,
    creature_ai_spell_test_config_like_cpp, creature_ai_test_spell_info_like_cpp,
    decode_creature_spell_full_log_like_cpp, decode_creature_spell_wire_header_like_cpp,
    faction_template_entry, represented_creature_spell_test_attributes_like_cpp,
    spell_misc_entry_like_cpp, spell_range_entry_like_cpp,
};
// #1363 slice 3b: the session-free lifecycle builders moved there too.
pub(super) use wow_world_test_support::lifecycle::lifecycle_test_map_store_like_cpp;
pub(super) use wow_world_test_support::map_objects::{
    add_canonical_capture_point_delete_gameobject_like_cpp,
    add_canonical_lifecycle_gameobject_on_map,
    add_canonical_spell_focus_gameobject_on_map_like_cpp, add_canonical_test_creature,
    add_canonical_test_creature_indexed_on_map_with_level, add_canonical_test_creature_on_map,
    add_canonical_test_creature_on_map_with_world_state,
    add_canonical_test_creature_on_map_with_world_state_and_owner, add_canonical_test_gameobject,
    add_canonical_test_gameobject_on_map, add_canonical_test_pet,
    add_canonical_test_pet_with_number, add_canonical_test_pet_with_visible_aura,
    add_canonical_test_player_on_map, add_canonical_test_player_on_map_with_difficulty,
    add_canonical_visibility_on_destroy_gameobject_like_cpp,
    add_canonical_visual_despawn_gameobject_like_cpp, canonical_party_type_for_test,
    shared_map_manager, test_creature_create_data, test_creature_guid, test_gameobject_guid,
    test_pet_guid, test_vehicle_guid,
};
pub(super) use wow_world_test_support::packets::{
    QuestGiverRequestItemsSummaryLikeCpp, drain_server_opcodes, drain_server_packet_bytes,
    expected_active_player_farsight_object_values_update_like_cpp,
    expected_dynamic_object_create_packet_like_cpp, packet_contains_quest_ids_in_order,
    party_update_sequence_num_like_cpp, quest_giver_request_items_summary_like_cpp,
    quest_list_level_fields_like_cpp, update_object_packet_count_like_cpp,
};
pub(super) use wow_world_test_support::spell_catalog::{
    difficulty_entry, test_pet_default_spell_store_like_cpp, test_pet_family_spell_store_like_cpp,
    test_pet_levelup_spell_store_like_cpp, test_serverside_spell_store_like_cpp,
    test_skill_line_entry_like_cpp, test_skill_race_class_info_like_cpp,
    test_spell_area_store_like_cpp, test_spell_custom_attribute_store_like_cpp,
    test_spell_group_stack_rule_store_like_cpp, test_spell_group_store_like_cpp,
    test_spell_learn_skill_rank_store_like_cpp, test_spell_learn_skill_store_like_cpp,
    test_spell_learn_spell_store_like_cpp, test_spell_linked_store_like_cpp,
    test_spell_pet_aura_store_like_cpp, test_spell_proc_entry_like_cpp,
    test_spell_required_store_like_cpp, test_spell_threat_entry_like_cpp,
    test_spell_totem_model_store_like_cpp,
};
