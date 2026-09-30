//! Borrowed handler catalogs and required session core ownership.

use std::sync::Arc;
use crate::{world_config_bool, world_config_f32, world_config_u32, player_regeneration_rates_like_cpp};
use crate::session_resources::SessionCoreCapabilitiesLikeCpp;
use super::session_handler_policies;

pub(super) fn build(
    object_mgr_catalogs: Arc<wow_world::session::ObjectMgrCatalogsLikeCpp>,
    player_grid_loader: wow_world::session::PlayerGridLoadResolverLikeCpp,
    geography: &super::geography_startup::GeographyBase,
    area_triggers: &super::area_trigger_template_startup::AreaTriggerWorldCatalogs,
    inventory: &super::inventory_catalogs::InventoryBaseCatalogs,
    item_auxiliary: &super::item_auxiliary_catalogs::ItemAuxiliaryCatalogs,
    player_creation: &super::player_creation_startup::PlayerCreationStartup,
    player_catalogs: &super::player_catalog_startup::PlayerCatalogs,
    skills: &super::skill_catalogs::SkillCatalogs,
    world_configs: &wow_config::WorldConfigSet,
    creature_runtime: &super::creature_catalog_startup::CreatureRuntimeCatalogs,
    creature_addon_store: &Arc<wow_data::CreatureAddonStoreLikeCpp>,
    world_spawns: &super::world_object_startup::WorldSpawnStartup,
    quest_rewards: &super::progression_catalog_startup::PlayerXpAndQuestRewards,
    battle_pet_selection_store: &Arc<wow_data::battle_pet_selection::BattlePetSelectionStoreLikeCpp>,
    presentation: &super::presentation_startup::PresentationCatalogs,
    condition_references: &super::condition_reference_startup::ConditionReferences,
    emotes_store: &Arc<wow_data::EmotesStore>,
    graveyard_store: &Arc<wow_data::GraveyardStore>,
    quest_admission: &super::quest_admission_startup::QuestAdmissionCatalogs,
    tact_key_store: &Arc<wow_data::TactKeyStore>,
    hotfix_blob_cache: &Arc<wow_data::HotfixBlobCache>,
    modules: &Arc<wow_module_api::ModuleRegistry>,
    id_generators: &wow_world::session::SessionIdGeneratorsLikeCpp,
    gameobject_template_lifecycle_store: &Arc<wow_data::GameObjectTemplateLifecycleStoreLikeCpp>,
    trainer_data_store: &Arc<wow_data::TrainerStoreLikeCpp>,
    instances: &super::world_instance_startup::WorldInstanceManagers,
    persistence: wow_world::session::SessionPersistencePortsLikeCpp,
) -> SessionCoreCapabilitiesLikeCpp {
    SessionCoreCapabilitiesLikeCpp {
            handler_catalogs: Arc::new(wow_world::session::SessionHandlerCatalogsLikeCpp {
                object_mgr: object_mgr_catalogs,
                player_grid_loader,
                area_triggers: Arc::new(wow_world::session::AreaTriggerCatalogsLikeCpp {
                    db2: Arc::clone(&geography.area_trigger_db2_store),
                    destinations: Arc::clone(&area_triggers.area_trigger_store),
                    scripts: Arc::clone(&area_triggers.area_trigger_script_store),
                    taverns: Arc::clone(&area_triggers.tavern_area_trigger_store),
                    script_dispatcher: None,
                }),
                item_valuation: Arc::new(wow_world::session::ItemValuationCatalogsLikeCpp {
                    import_prices: Arc::clone(&inventory.import_price_stores),
                    price_base: Arc::clone(&inventory.item_price_base_store),
                    item_classes: Arc::clone(&inventory.item_class_store),
                    currency_costs: Arc::clone(&inventory.item_currency_cost_store),
                    disenchant_loot: Arc::clone(&item_auxiliary.item_disenchant_loot_store),
                }),
                player_bootstrap: Arc::new(wow_world::session::PlayerBootstrapCatalogsLikeCpp {
                    create_info: Arc::clone(&player_creation.create_info),
                    glyph_properties: Arc::clone(&player_catalogs.glyph_properties_store),
                    talent_tabs: Arc::clone(&player_catalogs.talent_tab_store),
                    trait_node_entries: Arc::clone(&skills.trait_node_entry_store),
                    cast_spells: Arc::clone(&player_creation.cast_spells),
                    custom_spells: Arc::clone(&player_creation.custom_spells),
                    start_all_spells: world_config_bool(
                        world_configs,
                        "CONFIG_START_ALL_SPELLS",
                        false,
                    ),
                    start_all_explored: world_config_bool(
                        world_configs,
                        "CONFIG_START_ALL_EXPLORED",
                        false,
                    ),
                    start_all_reputation: world_config_bool(
                        world_configs,
                        "CONFIG_START_ALL_REP",
                        false,
                    ),
                }),
                player_rest_rates: Arc::new(wow_world::session::PlayerRestRatePolicyLikeCpp {
                    offline_wilderness: world_config_f32(
                        world_configs,
                        "RATE_REST_OFFLINE_IN_WILDERNESS",
                        1.0,
                    ),
                    offline_tavern_or_city: world_config_f32(
                        world_configs,
                        "RATE_REST_OFFLINE_IN_TAVERN_OR_CITY",
                        1.0,
                    ),
                    ingame: world_config_f32(world_configs, "RATE_REST_INGAME", 1.0),
                }),
                creature_spawns: Arc::new(wow_world::session::CreatureSpawnCatalogsLikeCpp {
                    difficulty: Arc::clone(&creature_runtime.creature_difficulty_store),
                    base_stats: Arc::clone(&creature_runtime.creature_base_stats_store),
                    health_rates: creature_runtime.creature_health_rates,
                    addons: Arc::clone(creature_addon_store),
                    equipment: Arc::clone(&world_spawns.creature_equipment_store),
                    power_types: Arc::clone(&player_catalogs.power_type_store),
                }),
                progression: Arc::new(wow_world::session::ProgressionCatalogsLikeCpp {
                    no_reset_talent_cost: world_config_bool(
                        world_configs,
                        "CONFIG_NO_RESET_TALENT_COST",
                        false,
                    ),
                    player_xp: Arc::clone(&quest_rewards.player_xp_table),
                    exploration_base_xp: Arc::clone(&quest_rewards.exploration_base_xp_store),
                    exploration_xp_rate: world_config_f32(world_configs, "RATE_XP_EXPLORE", 1.0),
                    min_discovered_scaled_xp_ratio: world_config_u32(
                        world_configs,
                        "CONFIG_MIN_DISCOVERED_SCALED_XP_RATIO",
                        0,
                    ),
                }),
                battle_pet_trainer_selection: Arc::clone(battle_pet_selection_store),
                quest_info: Arc::clone(&quest_rewards.quest_info_store),
                chat_policy: session_handler_policies::build_chat_policy(world_configs),
                group_invite_policy: session_handler_policies::build_group_invite_policy(
                    world_configs,
                ),
                support_feature_policy: session_handler_policies::build_support_feature_policy(
                    world_configs,
                    presentation.cfg_categories_store.as_ref(),
                ),
                player_regeneration_rates: Arc::new(player_regeneration_rates_like_cpp(
                    world_configs,
                )),
                bank_bag_slot_prices: Arc::clone(&inventory.bank_bag_slot_prices_store),
                adventure_map_pois: Arc::clone(&condition_references.adventure_map_poi_store),
                battlemaster_lists: Arc::clone(&condition_references.battlemaster_list_typed_store),
                emotes: Arc::clone(emotes_store),
                emotes_text: Arc::clone(&presentation.emotes_text_store),
                graveyards: Arc::clone(graveyard_store),
                lfg_dungeons: Arc::clone(&quest_admission.lfg_dungeon_store_like_cpp),
                tact_keys: Arc::clone(tact_key_store),
                hotfixes: Arc::clone(hotfix_blob_cache),
                modules: Arc::clone(modules),
                id_generators: Arc::new(wow_world::session::SessionIdGeneratorsLikeCpp {
                    player: Arc::clone(&id_generators.player),
                    item: Arc::clone(&id_generators.item),
                    equipment_set: Arc::clone(&id_generators.equipment_set),
                    void_storage_item: Arc::clone(&id_generators.void_storage_item),
                }),
            }),
            gameobject_template_lifecycle_store: Arc::clone(gameobject_template_lifecycle_store),
            persistence,
            trainer_store: Arc::clone(trainer_data_store),
            instance_lock_mgr: Arc::clone(&instances.instance_lock_mgr),
        }
}
