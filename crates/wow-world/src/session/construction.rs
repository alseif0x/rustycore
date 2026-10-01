// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Construction: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

#[cfg(test)]
use crate::session::state::AuraState;
#[cfg(test)]
use crate::session::state::BattlegroundState;
#[cfg(test)]
use crate::session::state::CollectionsState;
#[cfg(test)]
use crate::session::state::CombatState;
use crate::session::state::InstanceState;
use crate::session::state::InteractionState;
use crate::session::state::InventoryState;
use crate::session::state::LootState;
#[cfg(test)]
use crate::session::state::MovementState;
#[cfg(test)]
use crate::session::state::PetState;
#[cfg(test)]
use crate::session::state::PlayerIdentityState;
#[cfg(test)]
use crate::session::state::PlayerPresentationState;
#[cfg(test)]
use crate::session::state::ProgressionState;
use crate::session::state::SessionAccountState;
use crate::session::state::SessionAddonFilter;
use crate::session::state::SessionAdmissionState;
use crate::session::state::SessionCatalogs;
use crate::session::state::SessionCore;
use crate::session::state::SessionDirectory;
use crate::session::state::SessionDriverServices;
#[cfg(test)]
use crate::session::state::SessionFixtures;
use crate::session::state::SessionLifecycleState;
use crate::session::state::SessionPhaseRail;
use crate::session::state::SessionQuestState;
use crate::session::state::SessionRealmPolicy;
use crate::session::state::SessionSharedFlags;
use crate::session::state::SessionSocialLimits;
use crate::session::state::SessionSpellState;
use crate::session::state::SessionTransport;
use crate::session::state::SessionWorldConfig;
use crate::session::state::SessionWorldView;
#[cfg(test)]
use crate::session::state::TaxiVehicleState;
#[cfg(test)]
use crate::session::state::TeleportState;
use crate::session::state::VisibilityState;
use crate::session::state::WorldEntitiesState;

#[cfg(test)]
use super::BattlePetTestFixtureLikeCpp;
use super::DEFAULT_PLAYER_SAVE_INTERVAL_MS_LIKE_CPP;
use super::PlayerInteractionDataLikeCpp;
#[cfg(test)]
use super::empty_character_power_snapshot_like_cpp;
#[cfg(test)]
use super::instances::test_fixtures::InstanceTestFixtureLikeCpp;
#[cfg(test)]
use super::persistence::test_fixtures::LoadedPlayerFlagsTestFixtureLikeCpp;
#[cfg(test)]
use super::player_items::test_fixtures::PlayerItemTestFixtureLikeCpp;
#[cfg(test)]
use super::progression::PlayerSkillTestFixtureLikeCpp;
#[cfg(test)]
use super::quest::test_fixtures::QuestTestFixtureLikeCpp;
#[cfg(test)]
use super::rest_progression::RestMgrTestFixtureLikeCpp;
#[cfg(test)]
use super::social::test_fixtures::CalendarTestFixtureLikeCpp;
#[cfg(test)]
use super::social::test_fixtures::DuelTestFixtureLikeCpp;
#[cfg(test)]
use super::social::test_fixtures::GuildTestFixtureLikeCpp;
#[cfg(test)]
use super::social::test_fixtures::TradeTestFixtureLikeCpp;
#[cfg(test)]
use super::spell_state::PlayerSpellAndTraitTestFixtureLikeCpp;
#[cfg(test)]
use super::support_features::test_fixtures::SupportFeatureTestFixtureLikeCpp;
#[cfg(test)]
use super::test_support::test_fixtures::PlayerBootstrapCatalogTestFixtureLikeCpp;
use super::time_synchronization::TimeSynchronizationStateLikeCpp;
#[cfg(test)]
use super::visibility::test_fixtures::VisibilityTestFixtureLikeCpp;
use super::{Arc, AtomicBool, BTreeMap, BTreeSet};
use super::{ChatFloodConfigLikeCpp, ChatFloodThrottleDataLikeCpp, ChatLevelRequirementsLikeCpp};
use super::{ChatListenRangesLikeCpp, CreatureClassificationHealthRatesLikeCpp};
use super::{DurableItemLootPersistenceTrackerLikeCpp, DurableLootMoneyPersistenceTrackerLikeCpp};
use super::{Duration, HashMap, HashSet, Instant};
use super::{LegacyCreatureAggroConfigLikeCpp, LootDropRatesLikeCpp, MAX_SPECIALIZATIONS_LIKE_CPP};
use super::{MMapRuntimeConfigLikeCpp, MovementFlag, ObjectGuid};
use super::{PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP, PacketSpoofConfigLikeCpp, PetStable, PhaseShift};
use super::{RepresentedBattlePetSlotLikeCpp, ReputationRatesLikeCpp, Rng, RngCore, SeedableRng};
use super::{SessionState, SharedClientVisibleGuidsLikeCpp, SocketTimeoutsLikeCpp, StdRng};
use super::{UnitFlags, UnitMoveTypeLikeCpp, UnitStandStateType};
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
            core: SessionCore {
                account_id,

                account_name,
                security,
                expansion,
                account_expansion,
                build,
                locale,

                session_command_tx,
                session_command_rx,

                durable_creature_runtime_commands_like_cpp: Default::default(),

                state: SessionState::Authed,
                #[cfg(test)]
                driver_phase_trace_like_cpp: Vec::new(),
                player_registry: None,
                directory: SessionDirectory::default(),
                driver: SessionDriverServices {
                    time_synchronization: TimeSynchronizationStateLikeCpp::default(),
                    represented_runtime_rng_like_cpp: StdRng::from_entropy(),
                },
                flags: SessionSharedFlags {
                    advanced_combat_logging_enabled_like_cpp: Arc::new(AtomicBool::new(false)),
                    visibility_refresh_pending_like_cpp: Arc::new(AtomicBool::new(false)),
                },
                realm_policy: SessionRealmPolicy {
                    realm_battlegroup: 1,
                    realm_region: 1,
                    realm_names_like_cpp: BTreeMap::from([(
                        0x0101_0001,
                        ("RustyCore".to_string(), "RustyCore".to_string()),
                    )]),
                    realm_list_secret_like_cpp: [0; 32],
                    server_expansion_like_cpp: 2,
                    max_instances_per_hour_like_cpp: 5,
                    instance_ignore_level_like_cpp: false,
                    instance_ignore_raid_like_cpp: false,
                },
                account_state: SessionAccountState {
                    battlenet_account_id: account_id,
                    is_a_recruiter_like_cpp: false,
                    recruiter_id_like_cpp: 0,
                    legit_characters: Vec::new(),
                    recent_player_guid_low_like_cpp: 0,
                    mute_time_like_cpp: 0,
                },
                admission: SessionAdmissionState {
                    dispatch_table: build_dispatch_table(),
                    last_packet_time: Instant::now(),
                    last_phase_authority_like_cpp: [None, None],
                    map_phase_coordinated_like_cpp: false,
                    packet_spoof_config_like_cpp: PacketSpoofConfigLikeCpp::default(),
                    packet_throttling_like_cpp: HashMap::new(),
                    pending_packet_spoof_ban_like_cpp: None,
                    pending_packets: VecDeque::new(),
                    socket_timeout_deadline_like_cpp: Instant::now()
                        + Duration::from_secs(
                            SocketTimeoutsLikeCpp::default().unauthenticated_secs,
                        ),
                    socket_timeouts_like_cpp: SocketTimeoutsLikeCpp::default(),
                },
                transport: SessionTransport {
                    connection,
                    remote_address_like_cpp: None,
                    session_key,
                    session_mgr: None,
                },
                realm_id: 1,

                #[cfg(test)]
                guid_generator: None,
                #[cfg(test)]
                item_guid_generator_like_cpp: None,
                #[cfg(test)]
                equipment_set_guid_generator_like_cpp: None,
                #[cfg(test)]
                void_storage_item_id_generator_like_cpp: None,
                player_guid: None,

                #[cfg(test)]
                player_bootstrap_attached_like_cpp: false,
                current_map_id: 0,
                player_identity_bootstrap_like_cpp: None,
                map_manager: None,
                canonical_map_manager: None,
                player_handle_like_cpp: None,

                mmap_pathfinder_like_cpp: None,
                instance_lock_mgr: None,

                #[cfg(test)]
                module_registry_like_cpp: None,
                client_visible_guids_like_cpp: SharedClientVisibleGuidsLikeCpp::default(),
            },
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
                #[cfg(test)]
                represented_loot_roll_criteria_events: Vec::new(),
                represented_unique_gameobject_uses: std::collections::HashSet::new(),
                represented_gameobject_tap_lists: std::collections::HashMap::new(),
                #[cfg(test)]
                represented_locked_dungeon_encounters: std::collections::HashSet::new(),
                represented_personal_loot_money: std::collections::HashMap::new(),
                represented_personal_loot_owners: std::collections::HashSet::new(),
            },
            catalogs: SessionCatalogs {
                #[cfg(test)]
                player_bootstrap_catalog_test_fixture_like_cpp:
                    PlayerBootstrapCatalogTestFixtureLikeCpp::default(),

                trainer_store_like_cpp: None,
                #[cfg(test)]
                bank_bag_slot_prices_store: None,
                currency_types_store: None,
                #[cfg(test)]
                import_price_stores: None,
                #[cfg(test)]
                emotes_store: None,
                #[cfg(test)]
                emotes_text_store: None,
                #[cfg(test)]
                item_class_store: None,
                #[cfg(test)]
                item_currency_cost_store: None,
                items: crate::catalogs::item::ItemCatalogsLikeCpp::default(),
                trinity_string_store: None,
                heirloom_store: None,
                toy_store: None,
                combat_ratings_game_table: None,
                regen_game_tables: None,
                shield_block_regular_game_table: None,

                transmog_set_item_store: None,
                #[cfg(test)]
                item_price_base_store: None,
                player_stats: None,
                pvp_item_store: None,
                spell_catalogs: crate::catalogs::spell::SpellCatalogsLikeCpp::default(),
                durability_costs_store: None,
                durability_quality_store: None,
                item_template_addon_quest_log_item_ids_like_cpp: HashMap::new(),
                rand_prop_points_store: None,
                #[cfg(test)]
                item_disenchant_loot_store: None,
                loot_stores: None,
                condition_store: None,
                player_condition_store: None,
                #[cfg(test)]
                adventure_map_poi_store: None,
                content_tuning_store: None,
                curve_store: None,
                curve_point_store: None,
                scaling_stat_distribution_store: None,
                scaling_stat_values_store: None,
                disable_mgr: None,
                difficulty_store: None,
                lock_store: None,
                gem_properties_store: None,
                #[cfg(test)]
                tact_key_store: None,
                skill_store: None,
                trait_definition_store: None,
                trait_tree_skill_line_index: None,
                skill_line_store: None,
                skill_tiers_store: None,
                area_table_store: None,
                fishing_base_skill_store: None,
                #[cfg(test)]
                area_trigger_db2_store: None,
                #[cfg(test)]
                area_trigger_store: None,
                #[cfg(test)]
                area_trigger_script_store: None,
                #[cfg(test)]
                tavern_area_trigger_store: None,
                #[cfg(test)]
                graveyard_store: None,
                chr: crate::catalogs::chr::ChrCatalogsLikeCpp::default(),
                maps: crate::catalogs::map::MapCatalogsLikeCpp::default(),
                dungeon_encounter_store: None,
                world_safe_loc_store_like_cpp: None,
                access_requirement_store: None,
                lfg_dungeons_store: None,
                #[cfg(test)]
                lfg_dungeon_store_like_cpp: None,
                #[cfg(test)]
                battlemaster_list_store: None,
                factions: crate::catalogs::faction::FactionCatalogsLikeCpp::default(),
                friendship_rep_reaction_store: None,
                paragon_reputation_store: None,
                reputation_reward_rate_store: None,
                creatures: crate::catalogs::creature::CreatureCatalogsLikeCpp::default(),
                reputation_spillover_template_store: None,
                #[cfg(test)]
                creature_equipment_store_like_cpp: None,
                gameobjects: crate::catalogs::gameobject::GameObjectCatalogsLikeCpp::default(),
                #[cfg(test)]
                creature_addon_store_like_cpp: None,
                #[cfg(test)]
                creature_difficulty_store_like_cpp: None,
                #[cfg(test)]
                creature_base_stats_store_like_cpp: None,
                mount_store: None,
                mount_definition_store_like_cpp: None,
                mount_capability_store: None,
                mount_type_x_capability_store: None,
                mount_x_display_store: None,
                vehicle_store: None,
                vehicle_seat_store: None,
                #[cfg(test)]
                vehicle_template_store: None,
                vehicle_accessory_store: None,
                terrain_swap_store: None,
                phase_store: None,
                phase_group_store: None,

                talent_store: None,
                num_talents_at_level_store: None,
                power_type_store: None,
                cinematic_sequences_store: None,
                movie_store: None,
                script_name_interner: None,
                #[cfg(test)]
                object_mgr_catalogs_like_cpp: None,
                gameobject_template_lifecycle_store_like_cpp: None,
                quests: crate::catalogs::quest::QuestCatalogsLikeCpp::default(),
                quest_poi_store_like_cpp: None,
                #[cfg(test)]
                player_xp_table: None,
                #[cfg(test)]
                exploration_base_xp_store: None,
                waypoint_path_resolver_like_cpp: None,
            },
            config: SessionWorldConfig {
                #[cfg(test)]
                characters_per_realm_like_cpp: 60,
                #[cfg(test)]
                declined_names_used_like_cpp: false,
                #[cfg(test)]
                feature_system_bpay_store_enabled_like_cpp: false,
                #[cfg(test)]
                feature_system_character_undelete_enabled_like_cpp: false,

                legacy_creature_aggro_config_like_cpp: LegacyCreatureAggroConfigLikeCpp::default(),
                #[cfg(test)]
                area_trigger_script_dispatcher_like_cpp: None,
                #[cfg(test)]
                give_player_xp_script_dispatcher_like_cpp: None,
                #[cfg(test)]
                creature_health_rates_like_cpp: CreatureClassificationHealthRatesLikeCpp::default(),

                max_player_level_config_like_cpp: 80,
                max_primary_trade_skills_like_cpp:
                    crate::profession::DEFAULT_MAX_PRIMARY_TRADE_SKILLS_LIKE_CPP,
                represented_cast_unstuck_enabled_like_cpp: true,
                #[cfg(test)]
                exploration_xp_rate_like_cpp: 1.0,

                #[cfg(test)]
                min_discovered_scaled_xp_ratio_like_cpp: 0,
                loot_drop_rates: LootDropRatesLikeCpp::default(),
                reputation_rates: ReputationRatesLikeCpp::default(),
                repair_cost_rate_like_cpp: 1.0,
                durability_loss_on_death_rate_like_cpp: 0.1,
                stats_limits_like_cpp: wow_data::StatsLimitsLikeCpp::default(),
                reset_schedule_like_cpp: wow_instances::ResetSchedule::default(),

                vmap_indoor_check_like_cpp: false,
                enable_ae_loot_like_cpp: false,
                #[cfg(test)]
                addon_channel_like_cpp: true,
                #[cfg(test)]
                chat_fake_message_preventing_like_cpp: false,
                #[cfg(test)]
                party_raid_warnings_like_cpp: false,
                #[cfg(test)]
                allow_gm_group_like_cpp: false,
                #[cfg(test)]
                allow_two_side_interaction_group_like_cpp: false,
                #[cfg(test)]
                party_level_req_like_cpp: 1,
                #[cfg(test)]
                chat_strict_link_checking_kick_like_cpp: false,
                #[cfg(test)]
                chat_level_requirements_like_cpp: ChatLevelRequirementsLikeCpp::default(),
                #[cfg(test)]
                chat_listen_ranges_like_cpp: ChatListenRangesLikeCpp::default(),
                #[cfg(test)]
                chat_flood_config_like_cpp: ChatFloodConfigLikeCpp::default(),
                mmap_runtime_config_like_cpp: MMapRuntimeConfigLikeCpp::default(),
            },
            #[cfg(test)]
            fixtures: SessionFixtures {
                #[cfg(test)]
                identity: PlayerIdentityState {
                    #[cfg(test)]
                    player_race: 0,
                    #[cfg(test)]
                    player_class: 0,
                    #[cfg(test)]
                    player_level: 0,
                    #[cfg(test)]
                    player_gender: 0,
                    #[cfg(test)]
                    player_create_mode_like_cpp: wow_data::PLAYER_CREATE_MODE_NORMAL_LIKE_CPP,
                    #[cfg(test)]
                    player_name: None,
                    #[cfg(test)]
                    player_scale_duration_like_cpp: 0,
                    #[cfg(test)]
                    player_faction_template_like_cpp: None,
                    #[cfg(test)]
                    player_zone_id_like_cpp: 0,
                    #[cfg(test)]
                    player_area_id_like_cpp: 0,
                    #[cfg(test)]
                    player_zone_area_authority_complete_like_cpp: false,
                    #[cfg(test)]
                    represented_is_outdoors_like_cpp: None,
                },
                #[cfg(test)]
                collections: CollectionsState {
                    #[cfg(test)]
                    account_mounts_like_cpp: HashMap::new(),
                    #[cfg(test)]
                    represented_account_heirlooms_like_cpp: BTreeMap::new(),
                    #[cfg(test)]
                    represented_account_toys_like_cpp: BTreeMap::new(),
                    #[cfg(test)]
                    represented_item_appearances_like_cpp: HashSet::new(),
                    #[cfg(test)]
                    represented_item_appearance_blocks_like_cpp: Vec::new(),
                    #[cfg(test)]
                    represented_temporary_item_appearances_like_cpp: HashMap::new(),
                    #[cfg(test)]
                    represented_favorite_item_appearances_like_cpp: HashMap::new(),
                    #[cfg(test)]
                    represented_transmog_illusions_like_cpp: HashSet::new(),

                    #[cfg(test)]
                    represented_completed_achievements_like_cpp: HashSet::new(),
                },
                #[cfg(test)]
                auras: AuraState {
                    #[cfg(test)]
                    represented_shapeshift_form_like_cpp: 0,
                    #[cfg(test)]
                    player_spell_hit_aura_authority_tombstoned_like_cpp: false,
                    #[cfg(test)]
                    visible_auras: HashMap::new(),
                    #[cfg(test)]
                    player_aura_authority_complete_like_cpp: false,
                    #[cfg(test)]
                    canonical_threat_aura_snapshots_like_cpp: HashMap::new(),
                },
                #[cfg(test)]
                progression: ProgressionState {
                    #[cfg(test)]
                    championing_faction_like_cpp: 0,
                    #[cfg(test)]
                    represented_enchanting_skill: 0,
                    #[cfg(test)]
                    player_skill_test_fixture_like_cpp: PlayerSkillTestFixtureLikeCpp::default(),
                    #[cfg(test)]
                    represented_gray_level_script_overrides_like_cpp: HashMap::new(),
                    #[cfg(test)]
                    rest_mgr_test_fixture_like_cpp: RestMgrTestFixtureLikeCpp::default(),
                    #[cfg(test)]
                    represented_talent_reset_cost_like_cpp: 0,
                    #[cfg(test)]
                    represented_talent_reset_time_secs_like_cpp: 0,
                    #[cfg(test)]
                    player_character_points_like_cpp: 0,
                    #[cfg(test)]
                    player_xp: 0,
                    #[cfg(test)]
                    player_next_level_xp: 400,
                    #[cfg(test)]
                    represented_primary_specialization_id_like_cpp: 0,
                    #[cfg(test)]
                    represented_weapon_proficiency_like_cpp: 0,
                    #[cfg(test)]
                    represented_armor_proficiency_like_cpp: 0,
                    #[cfg(test)]
                    represented_confirm_respec_wipe_requests_like_cpp: Vec::new(),
                    #[cfg(test)]
                    represented_talent_reset_script_hooks_like_cpp: Vec::new(),
                    #[cfg(test)]
                    represented_talent_respec_visual_spell_casts_like_cpp: Vec::new(),
                    #[cfg(test)]
                    represented_talent_respec_criteria_events_like_cpp: Vec::new(),
                    #[cfg(test)]
                    represented_active_talent_group_like_cpp: 0,
                    #[cfg(test)]
                    represented_bonus_talent_groups_like_cpp: 0,
                    #[cfg(test)]
                    represented_talents_like_cpp: std::array::from_fn(|_| BTreeMap::new()),
                    #[cfg(test)]
                    represented_talents_loaded_like_cpp: false,
                    #[cfg(test)]
                    represented_glyphs_like_cpp: [[0;
                        wow_packet::packets::misc::MAX_GLYPH_SLOT_INDEX_LIKE_CPP];
                        MAX_SPECIALIZATIONS_LIKE_CPP],
                    #[cfg(test)]
                    represented_glyphs_loaded_like_cpp: false,
                    #[cfg(test)]
                    #[cfg(test)]
                    reputation_state_like_cpp: wow_entities::PlayerReputationStateLikeCpp::default(
                    ),
                    #[cfg(test)]
                    watched_faction_index_like_cpp: -1,
                },
                #[cfg(test)]
                combat: CombatState {
                    #[cfg(test)]
                    represented_player_powers_like_cpp: empty_character_power_snapshot_like_cpp(),
                    #[cfg(test)]
                    represented_player_max_powers_like_cpp: empty_character_power_snapshot_like_cpp(
                    ),
                    #[cfg(test)]
                    represented_player_base_mana_like_cpp: 0,
                    #[cfg(test)]
                    selection_guid: None,
                    #[cfg(test)]
                    combat_target: None,
                    #[cfg(test)]
                    in_combat: false,
                    #[cfg(test)]
                    player_alive_like_cpp: true,
                    #[cfg(test)]
                    player_game_master_like_cpp: false,
                    #[cfg(test)]
                    player_cheat_god_like_cpp: false,
                    #[cfg(test)]
                    player_normal_damage_immune_like_cpp: false,
                    #[cfg(test)]
                    player_environmental_damage_immune_like_cpp: false,
                    #[cfg(test)]
                    player_health_like_cpp: 100,
                    #[cfg(test)]
                    player_max_health_like_cpp: 100,
                    #[cfg(test)]
                    area_spirit_healer_guid_like_cpp: ObjectGuid::EMPTY,
                    #[cfg(test)]
                    player_pvp_hostile_like_cpp: false,
                    #[cfg(test)]
                    player_pvp_enabled_like_cpp: false,
                    #[cfg(test)]
                    player_in_pvp_flag_like_cpp: false,
                    #[cfg(test)]
                    player_pvp_end_timer_like_cpp: None,
                    #[cfg(test)]
                    player_contested_pvp_timer_like_cpp: 0,
                    #[cfg(test)]
                    represented_resurrection_request_like_cpp: None,
                    #[cfg(test)]
                    represented_delayed_resurrection_after_teleport_like_cpp: None,
                    #[cfg(test)]
                    represented_death_timer_active_like_cpp: false,
                    #[cfg(test)]
                    represented_repop_at_graveyard_count: 0,
                },
                #[cfg(test)]
                movement: MovementState {
                    #[cfg(test)]
                    player_position: None,
                    #[cfg(test)]
                    player_movement_flags_like_cpp: MovementFlag::NONE,
                    #[cfg(test)]
                    represented_can_swim_to_fly_transition_like_cpp: false,
                    #[cfg(test)]
                    represented_mover_fixed_position_vehicle_like_cpp: false,
                    #[cfg(test)]
                    player_movement_time_like_cpp: 0,
                    #[cfg(test)]
                    player_movement_jump_like_cpp: wow_packet::packets::movement::JumpInfo::default(
                    ),
                    #[cfg(test)]
                    last_fall_time_like_cpp: 0,
                    #[cfg(test)]
                    last_fall_z_like_cpp: 0.0,
                    #[cfg(test)]
                    fall_damage_events_like_cpp: Vec::new(),
                    #[cfg(test)]
                    player_out_of_bounds_like_cpp: false,
                    #[cfg(test)]
                    under_map_damage_events_like_cpp: Vec::new(),
                    #[cfg(test)]
                    movement_jump_proc_requests_like_cpp: 0,

                    #[cfg(test)]
                    player_moved_unit_guid_like_cpp: ObjectGuid::EMPTY,

                    #[cfg(test)]
                    movement_ack_events_like_cpp: Vec::new(),
                    #[cfg(test)]
                    represented_vehicle_dismiss_movements_like_cpp: Vec::new(),
                    #[cfg(test)]
                    represented_vehicle_base_movements_like_cpp: Vec::new(),
                    #[cfg(test)]
                    movement_counter_like_cpp: 0,
                    #[cfg(test)]
                    player_collision_height_like_cpp: 1.0,
                    #[cfg(test)]
                    delayed_operations_processed_like_cpp: 0,
                    #[cfg(test)]
                    forced_speed_changes_like_cpp: [0; UnitMoveTypeLikeCpp::COUNT],
                    #[cfg(test)]
                    movement_speed_rates_like_cpp: [1.0; UnitMoveTypeLikeCpp::COUNT],
                    #[cfg(test)]
                    movement_force_mod_magnitude_changes_like_cpp: 0,
                    #[cfg(test)]
                    movement_force_mod_magnitude_like_cpp: 1.0,
                    #[cfg(test)]
                    movement_speed_ack_events_like_cpp: Vec::new(),
                },
                #[cfg(test)]
                teleport: TeleportState {
                    #[cfg(test)]
                    move_spline_done_taxi_events_like_cpp: Vec::new(),
                    #[cfg(test)]
                    represented_can_delay_teleport_like_cpp: false,
                    #[cfg(test)]
                    represented_has_delayed_teleport_like_cpp: false,
                    #[cfg(test)]
                    near_teleport_pending_like_cpp: false,
                    #[cfg(test)]
                    represented_far_teleport_pending_like_cpp: false,
                    #[cfg(test)]
                    near_teleport_destination_like_cpp: None,
                    #[cfg(test)]
                    represented_delayed_teleport_like_cpp: None,
                    #[cfg(test)]
                    near_teleport_destination_zone_area_like_cpp: None,
                    #[cfg(test)]
                    represented_homebind_like_cpp: None,
                    #[cfg(test)]
                    move_teleport_ack_events_like_cpp: Vec::new(),
                    #[cfg(test)]
                    pending_teleport: None,
                },
                #[cfg(test)]
                vehicles: TaxiVehicleState {
                    #[cfg(test)]
                    taxi_destinations_like_cpp: Vec::new(),
                    #[cfg(test)]
                    represented_activate_taxi_requests_like_cpp: Vec::new(),
                    #[cfg(test)]
                    taxi_flight_state_like_cpp: None,
                    #[cfg(test)]
                    taxi_unit_flags_like_cpp: UnitFlags::empty(),
                    #[cfg(test)]
                    taxi_mounted_like_cpp: false,
                    #[cfg(test)]
                    player_mount_display_id_like_cpp: 0,
                    #[cfg(test)]
                    player_mount_vehicle_id_like_cpp: 0,
                    #[cfg(test)]
                    player_mount_vehicle_kit_like_cpp: None,
                    #[cfg(test)]
                    player_mount_vehicle_accessories_like_cpp: Vec::new(),
                    #[cfg(test)]
                    player_mount_vehicle_seat_count_like_cpp: 0,
                    #[cfg(test)]
                    player_mount_vehicle_usable_seat_count_like_cpp: 0,
                    #[cfg(test)]
                    player_vehicle_seat_flags_like_cpp: None,
                    #[cfg(test)]
                    player_vehicle_seat_id_like_cpp: None,
                    #[cfg(test)]
                    represented_vehicle_seat_change_requests_like_cpp: Vec::new(),
                    #[cfg(test)]
                    represented_vehicle_seat_spell_click_requests_like_cpp: Vec::new(),
                    #[cfg(test)]
                    represented_vehicle_enter_requests_like_cpp: Vec::new(),
                    #[cfg(test)]
                    mount_vehicle_create_requests_like_cpp: 0,
                    #[cfg(test)]
                    mount_vehicle_remove_requests_like_cpp: 0,
                    #[cfg(test)]
                    mount_cancel_expected_vehicle_aura_packets_like_cpp: 0,
                    #[cfg(test)]
                    mount_collision_height_update_requests_like_cpp: 0,
                    #[cfg(test)]
                    player_mounted_like_cpp: false,
                    #[cfg(test)]
                    player_on_transport_like_cpp: false,
                    #[cfg(test)]
                    player_transport_login_state_like_cpp: None,
                },
                #[cfg(test)]
                pets: PetState {
                    #[cfg(test)]
                    temporary_pet_unsummon_requests_like_cpp: 0,
                    #[cfg(test)]
                    represented_pet_guid_like_cpp: None,
                    #[cfg(test)]
                    represented_temporary_unsummoned_pet_number_like_cpp: 0,
                    #[cfg(test)]
                    represented_old_pet_spell_like_cpp: 0,
                    #[cfg(test)]
                    represented_pet_stable_like_cpp: PetStable::default(),
                    #[cfg(test)]
                    represented_character_pet_rows_empty_authority_complete_like_cpp: false,

                    #[cfg(test)]
                    represented_pet_created_by_spell_like_cpp: 0,
                    #[cfg(test)]
                    represented_pet_react_state_like_cpp:
                        wow_packet::packets::pet::REACT_DEFENSIVE_LIKE_CPP,
                    #[cfg(test)]
                    represented_pet_command_state_like_cpp:
                        wow_packet::packets::pet::COMMAND_FOLLOW_LIKE_CPP,
                    #[cfg(test)]
                    temporary_mount_pet_react_state_like_cpp: None,
                    #[cfg(test)]
                    mount_pet_control_disable_requests_like_cpp: 0,
                    #[cfg(test)]
                    mount_pet_control_enable_requests_like_cpp: 0,
                    #[cfg(test)]
                    mount_pet_resummon_requests_like_cpp: 0,
                    #[cfg(test)]
                    temporary_pet_resummon_requests_like_cpp: 0,
                    #[cfg(test)]
                    represented_pet_movement_speed_rates_like_cpp: [1.0;
                        UnitMoveTypeLikeCpp::COUNT],
                    #[cfg(test)]
                    represented_pet_speed_propagations_like_cpp: 0,
                    #[cfg(test)]
                    battle_pet_test_fixture_like_cpp: BattlePetTestFixtureLikeCpp::default(),
                },
                #[cfg(test)]
                battleground: BattlegroundState {
                    #[cfg(test)]
                    represented_arena_team_id_invited_like_cpp: 0,
                    #[cfg(test)]
                    represented_wargame_invite_acceptances_like_cpp: Vec::new(),
                    #[cfg(test)]
                    player_battleground_type_id_like_cpp: None,
                    #[cfg(test)]
                    player_battleground_map_id_like_cpp: None,
                    #[cfg(test)]
                    represented_battleground_status_like_cpp: None,
                    #[cfg(test)]
                    represented_battleground_leave_requests_like_cpp: 0,
                    #[cfg(test)]
                    represented_battlemaster_hellos_like_cpp: Vec::new(),
                    #[cfg(test)]
                    represented_battlefield_lists_like_cpp: Vec::new(),
                    #[cfg(test)]
                    represented_battlemaster_joins_like_cpp: Vec::new(),
                    #[cfg(test)]
                    represented_battlemaster_join_arenas_like_cpp: Vec::new(),
                    #[cfg(test)]
                    represented_battlemaster_join_skirmishes_like_cpp: Vec::new(),
                    #[cfg(test)]
                    represented_battleground_queue_slots_like_cpp: Vec::new(),
                    #[cfg(test)]
                    represented_battlefield_ports_like_cpp: Vec::new(),
                },
                #[cfg(test)]
                presentation: PlayerPresentationState {
                    #[cfg(test)]
                    cuf_profiles_like_cpp: vec![
                        None;
                        wow_packet::packets::misc::MAX_CUF_PROFILES_LIKE_CPP
                    ],
                    #[cfg(test)]
                    cuf_profiles_loaded_like_cpp: false,
                    #[cfg(test)]
                    player_stand_state_like_cpp: UnitStandStateType::Stand,
                    #[cfg(test)]
                    represented_live_applications_like_cpp: Vec::new(),
                    #[cfg(test)]
                    player_emote_state_like_cpp: 0,
                    #[cfg(test)]
                    active_player_local_flags_like_cpp: 0,
                    #[cfg(test)]
                    active_player_transport_server_time_like_cpp: 0,
                    #[cfg(test)]
                    active_player_multi_action_bars_like_cpp: 0,
                    #[cfg(test)]
                    represented_action_buttons_like_cpp: [0;
                        wow_packet::packets::misc::MAX_ACTION_BUTTONS],
                    #[cfg(test)]
                    represented_action_buttons_loaded_like_cpp: false,
                    #[cfg(test)]
                    represented_alter_appearance_requests_like_cpp: Vec::new(),
                    #[cfg(test)]
                    represented_confirm_barbers_choice_requests_like_cpp: Vec::new(),
                    #[cfg(test)]
                    player_object_scale_like_cpp: 1.0,
                    #[cfg(test)]
                    player_unit_flags_like_cpp: UnitFlags::PLAYER_CONTROLLED,
                    #[cfg(test)]
                    represented_cinematic_state_like_cpp:
                        wow_entities::PlayerCinematicStateLikeCpp::default(),
                    #[cfg(test)]
                    #[cfg(test)]
                    #[cfg(test)]
                    represented_cinematic_next_camera_events_like_cpp: Vec::new(),
                    #[cfg(test)]
                    represented_cinematic_end_events_like_cpp: Vec::new(),
                    #[cfg(test)]
                    #[cfg(test)]
                    represented_movie_complete_events_like_cpp: Vec::new(),
                },
            },
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
                #[cfg(test)]
                represented_transmog_criteria_events: Vec::new(),
            },
            spell_state: SessionSpellState {
                legacy_spell_script_spell_ids_like_cpp: None,
                spell_linked_rejected_trigger_spell_ids_like_cpp: None,
                spell_script_all_rank_root_spell_ids_like_cpp: None,
                spell_script_exact_spell_ids_like_cpp: None,
                represented_offhand_check_at_spell_unlearn_like_cpp: true,
                represented_spell_execute_log_effects_like_cpp: Vec::new(),
                spell_acquisition_cast_authority_like_cpp: None,
                spell_acquisition_craft_authority_like_cpp: None,
                #[cfg(test)]
                player_spell_test_fixture_like_cpp: PlayerSpellAndTraitTestFixtureLikeCpp::default(
                ),
                #[cfg(test)]
                represented_spell_acquisition_post_commit_actions_like_cpp: Vec::new(),
                #[cfg(test)]
                represented_spell_history_packets_like_cpp: (Vec::new(), Vec::new()),
                #[cfg(test)]
                represented_self_res_spells_like_cpp: BTreeSet::new(),
                #[cfg(test)]
                represented_override_spells_like_cpp: HashMap::new(),
                #[cfg(test)]
                represented_override_spells_complete_like_cpp: false,
                #[cfg(test)]
                active_spell_cast: None,
                #[cfg(test)]
                represented_pending_spell_cast_request_like_cpp: None,
                #[cfg(test)]
                last_spell_cast_time: None,
                #[cfg(test)]
                last_spell_cast_time_per_spell: HashMap::new(),
                #[cfg(test)]
                represented_character_spell_cooldowns_like_cpp: HashMap::new(),
                #[cfg(test)]
                represented_character_spell_cooldowns_loaded_like_cpp: false,
                #[cfg(test)]
                represented_character_spell_charges_like_cpp: BTreeMap::new(),
                #[cfg(test)]
                represented_character_spell_charges_loaded_like_cpp: false,
            },
            social: SessionSocialLimits {
                max_recruit_a_friend_bonus_player_level_like_cpp: 85,
                max_recruit_a_friend_bonus_player_level_difference_like_cpp: 4,
                chat_flood_data_like_cpp: [ChatFloodThrottleDataLikeCpp::default(); 2],
                addon_filter: SessionAddonFilter::default(),
                #[cfg(test)]
                group_guid: None,
                #[cfg(test)]
                represented_subgroup_like_cpp: None,
                #[cfg(test)]
                represented_group_update_sequences_like_cpp: std::array::from_fn(|_| {
                    Default::default()
                }),
                #[cfg(test)]
                guild_test_fixture_like_cpp: GuildTestFixtureLikeCpp::default(),
                #[cfg(test)]
                calendar_test_fixture_like_cpp: CalendarTestFixtureLikeCpp::default(),
                #[cfg(test)]
                trade_test_fixture_like_cpp: TradeTestFixtureLikeCpp::default(),
                #[cfg(test)]
                represented_sign_petitions_like_cpp: Vec::new(),
                #[cfg(test)]
                represented_decline_petitions_like_cpp: Vec::new(),
                #[cfg(test)]
                represented_query_petitions_like_cpp: Vec::new(),
                #[cfg(test)]
                represented_silence_party_talker_like_cpp: Vec::new(),
                #[cfg(test)]
                duel_test_fixture_like_cpp: DuelTestFixtureLikeCpp::default(),
            },
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
            interaction: InteractionState {
                vendor_item_counts: HashMap::new(),
                #[cfg(test)]
                vendor_buy_item_test_override_like_cpp: None,
                #[cfg(test)]
                support_feature_test_fixture_like_cpp: SupportFeatureTestFixtureLikeCpp::default(),
                #[cfg(test)]
                player_interaction_data_like_cpp: PlayerInteractionDataLikeCpp::default(),
                #[cfg(test)]
                gossip_options: Vec::new(),
            },
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
            },
        }
    }

    #[cfg(test)]
    pub(crate) fn seed_represented_runtime_rng_like_cpp(&mut self, seed: u64) {
        self.core.driver.represented_runtime_rng_like_cpp = StdRng::seed_from_u64(seed);
    }

    pub(crate) fn represented_urand_u32_like_cpp(&mut self, min: u32, max: u32) -> u32 {
        self.core.represented_urand_u32_like_cpp(min, max)
    }

    pub(crate) fn represented_runtime_subrng_like_cpp(&mut self) -> StdRng {
        self.core.represented_runtime_subrng_like_cpp()
    }
}

impl crate::session::state::SessionCore {
    pub(crate) fn represented_urand_u32_like_cpp(&mut self, min: u32, max: u32) -> u32 {
        if min >= max {
            return min;
        }
        self.driver
            .represented_runtime_rng_like_cpp
            .gen_range(min..=max)
    }

    pub(crate) fn represented_runtime_subrng_like_cpp(&mut self) -> StdRng {
        StdRng::seed_from_u64(self.driver.represented_runtime_rng_like_cpp.next_u64())
    }
}
