// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Construction: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use crate::session::state::InstanceState;
use crate::session::state::InteractionState;
use crate::session::state::InventoryState;
use crate::session::state::LootState;
use crate::session::state::SessionCatalogs;
use crate::session::state::SessionCore;
#[cfg(any(test, feature = "test-fixtures"))]
use crate::session::state::SessionFixtures;
use crate::session::state::SessionLifecycleState;
use crate::session::state::SessionPhaseRail;
use crate::session::state::SessionQuestState;
use crate::session::state::SessionSocialLimits;
use crate::session::state::SessionSpellState;
use crate::session::state::SessionWorldConfig;
use crate::session::state::SessionWorldView;
use crate::session::state::VisibilityState;
use crate::session::state::WorldEntitiesState;

#[cfg(test)]
use super::BTreeMap;
use super::DEFAULT_PLAYER_SAVE_INTERVAL_MS_LIKE_CPP;
use super::ObjectGuid;
use super::PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP;
#[cfg(test)]
use super::SeedableRng;
#[cfg(test)]
use super::StdRng;
#[cfg(test)]
use super::instances::test_fixtures::InstanceTestFixtureLikeCpp;
#[cfg(test)]
use super::persistence::test_fixtures::LoadedPlayerFlagsTestFixtureLikeCpp;
#[cfg(test)]
use super::player_items::test_fixtures::PlayerItemTestFixtureLikeCpp;
#[cfg(test)]
use super::quest::test_fixtures::QuestTestFixtureLikeCpp;
#[cfg(test)]
use super::visibility::test_fixtures::VisibilityTestFixtureLikeCpp;
use super::{Arc, BTreeSet};
use super::{DurableItemLootPersistenceTrackerLikeCpp, DurableLootMoneyPersistenceTrackerLikeCpp};
use super::{HashMap, Instant};
use super::{PhaseShift, RepresentedBattlePetSlotLikeCpp};
use super::{VecDeque, WorldPacket, WorldSession, build_dispatch_table, connection};
use super::{default_account_data_like_cpp, lifecycle};

impl WorldSession {
    pub(in crate::session) const MIN_ITEM_LEVEL_LIKE_CPP: u32 = 1;
    pub(in crate::session) const MAX_ITEM_LEVEL_LIKE_CPP: u32 = 1300;

    /// Create a new session with the given account info and channels.
    pub fn new(
        account_id: u32,
        account_name: String,
        security: u8,
        expansion: u8,
        account_expansion: u8,
        build: u32,
        session_key: Vec<u8>,
        locale: String,
        packet_rx: flume::Receiver<WorldPacket>,
        send_tx: flume::Sender<Vec<u8>>,
    ) -> Self {
        let (session_command_tx, session_command_rx) = flume::bounded(256);
        // One outstanding request per phase at most: the producer waits for the
        // completion boundary of each pass before issuing the next one, so a
        // full rail means a producer that did not wait.
        let (session_phase_tx, session_phase_rx) = flume::bounded(2);

        // The instance endpoint keeps the pre-#297 default; the kernel does not
        // hardcode a world-server address of its own.
        let mut connection = wow_session::SessionConnection::new(send_tx, packet_rx);
        connection.set_instance_endpoint([127, 0, 0, 1], 8086);

        Self {
            core: SessionCore::new_for_world_session_like_cpp(
                account_id,
                account_name,
                security,
                expansion,
                account_expansion,
                build,
                locale,
                session_command_tx,
                session_command_rx,
                connection,
                session_key,
            ),
            lifecycle: SessionLifecycleState {
                account_data_like_cpp: default_account_data_like_cpp(),
                battle_pet_account_attachment_like_cpp: None,
                character_rename_callbacks: Default::default(),
                durable_item_loot_persistence_like_cpp:
                    DurableItemLootPersistenceTrackerLikeCpp::default(),
                durable_loot_money_persistence_like_cpp: Arc::new(
                    DurableLootMoneyPersistenceTrackerLikeCpp::default(),
                ),
                finalization: None,
                homebind_persistence_tx_like_cpp: None,
                level_played_time: 0,
                login_time: None,
                logout_time: None,
                next_player_save_ms_like_cpp: DEFAULT_PLAYER_SAVE_INTERVAL_MS_LIKE_CPP,
                pending_periodic_player_save_like_cpp: false,
                persistence_ports_like_cpp: Box::default(),
                pet_load_query_holder_rows_like_cpp:
                    lifecycle::PetLoadQueryHolderRowsLikeCpp::default(),
                player_loading: None,
                player_login_claim_like_cpp: None,
                player_logout_like_cpp: false,
                player_save_interval_ms_like_cpp: DEFAULT_PLAYER_SAVE_INTERVAL_MS_LIKE_CPP,
                total_played_time: 0,
                tutorials_changed_like_cpp: false,
                tutorials_like_cpp: [0; 8],
                tutorials_loaded_coherently_like_cpp: false,
                tutorials_loaded_from_db_like_cpp: false,
                #[cfg(test)]
                player_flags_test_fixture_like_cpp: LoadedPlayerFlagsTestFixtureLikeCpp::default(),
                #[cfg(test)]
                represented_at_login_flags_like_cpp: 0,
                #[cfg(test)]
                represented_at_login_flag_removals_like_cpp: Vec::new(),
                #[cfg(test)]
                loot_money_persistence_test_result_like_cpp: None,
                #[cfg(test)]
                loaded_player_customizations_like_cpp: Box::default(),
            },
            phase: SessionPhaseRail {
                tx: session_phase_tx,
                rx: session_phase_rx,
            },
            loot: LootState {
                #[cfg(test)]
                pass_on_group_loot: false,
                #[cfg(test)]
                loot_specialization_id: 0,
                loot_table: std::collections::HashMap::new(),
                represented_loot_cache_generations_like_cpp: std::collections::HashMap::new(),
                active_loot_guid: ObjectGuid::EMPTY,
                active_loot_view_owners: std::collections::HashSet::new(),
                active_loot_view_generations_like_cpp: std::collections::HashMap::new(),
                active_loot_view_authorities_like_cpp: std::collections::HashMap::new(),
                represented_loot_rolls: std::collections::HashMap::new(),
                #[cfg(test)]
                loot_item_store_test_grants_like_cpp: None,
                #[cfg(test)]
                loot_item_store_test_success_like_cpp: true,
                #[cfg(test)]
                loot_item_store_test_commit_gate_like_cpp: None,
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_loot_roll_criteria_events: Vec::new(),
                represented_unique_gameobject_uses: std::collections::HashSet::new(),
                represented_gameobject_tap_lists: std::collections::HashMap::new(),
                #[cfg(test)]
                represented_locked_dungeon_encounters: std::collections::HashSet::new(),
                represented_personal_loot_money: std::collections::HashMap::new(),
                represented_personal_loot_owners: std::collections::HashSet::new(),
            },
            catalogs: SessionCatalogs::default(),
            config: SessionWorldConfig::default(),
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures: SessionFixtures::default(),
            inventory: InventoryState {
                #[cfg(test)]
                #[cfg(test)]
                represented_using_pvp_item_levels_like_cpp: false,
                #[cfg(test)]
                player_gold: 0,
                #[cfg(test)]
                player_item_test_fixture_like_cpp: PlayerItemTestFixtureLikeCpp::default(),
                #[cfg(test)]
                represented_bank_bag_slot_flags_like_cpp: [0; 7],
                #[cfg(test)]
                represented_bank_item_moves_like_cpp: Vec::new(),
                #[cfg(test)]
                represented_guild_bank_inventory_moves_like_cpp: Vec::new(),
                #[cfg(test)]
                represented_guild_bank_list_requests_like_cpp: Vec::new(),
                #[cfg(test)]
                represented_guild_bank_money_moves_like_cpp: Vec::new(),
                #[cfg(test)]
                represented_guild_bank_tab_actions_like_cpp: Vec::new(),
                #[cfg(test)]
                represented_auction_replicate_requests_like_cpp: Vec::new(),
                #[cfg(test)]
                represented_auction_place_bids_like_cpp: Vec::new(),
                #[cfg(test)]
                represented_auction_remove_items_like_cpp: Vec::new(),
                #[cfg(test)]
                represented_auction_sell_items_like_cpp: Vec::new(),
                #[cfg(test)]
                represented_auto_unequip_offhand_requests_like_cpp: Vec::new(),
                represented_guild_repair_bank_state_like_cpp: None,
                #[cfg(test)]
                represented_guild_repair_bank_withdraws_like_cpp: Vec::new(),
                #[cfg(test)]
                player_currencies: HashMap::new(),

                #[cfg(test)]
                inventory_item_objects: HashMap::new(),
                #[cfg(test)]
                represented_equipment_sets_like_cpp:
                    wow_entities::PlayerEquipmentSetsLikeCpp::default(),
                #[cfg(test)]
                represented_void_storage_items_like_cpp: std::array::from_fn(|_| None),
                #[cfg(test)]
                represented_void_storage_loaded_like_cpp: false,
                #[cfg(test)]
                player_equipment_inventory_authority_complete_like_cpp: false,
                #[cfg(any(test, feature = "test-fixtures"))]
                represented_transmog_criteria_events: Vec::new(),
            },
            spell_state: SessionSpellState::new_like_cpp(),
            social: SessionSocialLimits::with_recruit_a_friend_limits_like_cpp(85, 4),
            instances: InstanceState {
                #[cfg(test)]
                instance_test_fixture_like_cpp: InstanceTestFixtureLikeCpp::default(),
                #[cfg(test)]
                represented_adventure_map_start_quest_requests_like_cpp: Vec::new(),
                #[cfg(test)]
                represented_instance_reset_times_like_cpp: BTreeMap::new(),

                #[cfg(test)]
                represented_explored_zones_like_cpp: [0; PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP],
                #[cfg(test)]
                represented_reveal_world_map_overlay_criteria_like_cpp: Vec::new(),
                #[cfg(test)]
                represented_area_zone_criteria_like_cpp: Vec::new(),
                pending_bind: None,
                #[cfg(test)]
                represented_confirmed_pending_binds: Vec::new(),
            },
            world_entities: WorldEntitiesState {
                represented_creature_auras_like_cpp: Vec::new(),

                pending_creature_spawn: None,
                pending_creature_kill_loot_like_cpp: Vec::new(),
                pending_creature_kill_rewards_like_cpp: Vec::new(),
                #[cfg(test)]
                represented_creature_kill_events_like_cpp: Vec::new(),
                creature_tick: 0,
                #[cfg(test)]
                represented_gameobject_criteria_events: Vec::new(),
                represented_gameobject_use_effects: Vec::new(),
                represented_gameobject_use_states: std::collections::BTreeMap::new(),
                suppress_creature_movement_queued_at_or_before_like_cpp: None,
                represented_gameobject_phase_shifts: std::collections::HashMap::new(),
            },
            visibility: VisibilityState {
                client_visible_transports_like_cpp: Default::default(),
                #[cfg(test)]
                visibility_test_fixture_like_cpp: VisibilityTestFixtureLikeCpp::default(),
                last_observed_farsight_object_like_cpp: wow_core::ObjectGuid::EMPTY,
                represented_dynamic_object_values_updates_delivered_like_cpp:
                    std::collections::HashSet::new(),
                represented_player_unit_values_updates_delivered_like_cpp:
                    std::collections::HashSet::new(),
                represented_gameobject_visual_despawns_delivered_like_cpp:
                    std::collections::HashSet::new(),
                represented_capture_point_removed_delivered_like_cpp:
                    std::collections::HashSet::new(),
                last_visibility_pos: None,
            },
            interaction: InteractionState::new_like_cpp(),
            quest_state: SessionQuestState {
                min_quest_scaled_xp_ratio_like_cpp: 0,
                quest_high_level_hide_diff_like_cpp: 7,
                quest_low_level_hide_diff_like_cpp: 4,
                represented_quest_complete_status_updates_like_cpp: Vec::new(),
                represented_quest_objective_progress_draining_like_cpp: false,
                represented_quest_objective_progress_events_like_cpp: VecDeque::new(),
                movement_visibility_refresh_requests_like_cpp: 0,
                #[cfg(test)]
                quest_test_fixture_like_cpp: QuestTestFixtureLikeCpp::default(),
            },
            view: SessionWorldView {
                is_pvp_realm_like_cpp: false,
                is_ffa_pvp_realm_like_cpp: false,
                combat_tick_last_at_like_cpp: Instant::now(),
                last_presented_creature_melee_health_state_revision_like_cpp: 0,
                taxi_node_map_ids_like_cpp: HashMap::new(),
                active_area_trigger: None,
                #[cfg(test)]
                area_trigger_script_dispatcher_like_cpp: None,
            },
            dispatch_table: build_dispatch_table(),
        }
    }

    #[cfg(test)]
    pub(crate) fn seed_represented_runtime_rng_like_cpp(&mut self, seed: u64) {
        self.core.driver.represented_runtime_rng_like_cpp = StdRng::seed_from_u64(seed);
    }
}
