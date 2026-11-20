//! Immutable player bootstrap and progression policy catalogs.

use super::*;

pub(super) fn build_player_bootstrap(
    player_create_info_store: &Arc<wow_data::PlayerCreateInfoStoreLikeCpp>,
    glyph_properties_store: &Arc<wow_data::GlyphPropertiesStore>,
    talent_tab_store: &Arc<wow_data::TalentTabStore>,
    trait_node_entry_store: &Arc<wow_data::trait_tree::TraitNodeEntryStore>,
    player_create_cast_spell_store: &Arc<wow_data::PlayerCreateInfoCastSpellStoreLikeCpp>,
    player_create_custom_spell_store: &Arc<wow_data::PlayerCreateInfoCustomSpellStoreLikeCpp>,
    world_configs: &wow_config::WorldConfigSet,
) -> Arc<wow_world::session::PlayerBootstrapCatalogsLikeCpp> {
    Arc::new(wow_world::session::PlayerBootstrapCatalogsLikeCpp {
        create_info: Arc::clone(player_create_info_store),
        glyph_properties: Arc::clone(glyph_properties_store),
        talent_tabs: Arc::clone(talent_tab_store),
        trait_node_entries: Arc::clone(trait_node_entry_store),
        cast_spells: Arc::clone(player_create_cast_spell_store),
        custom_spells: Arc::clone(player_create_custom_spell_store),
        start_all_spells: world_config_bool(world_configs, "CONFIG_START_ALL_SPELLS", false),
        start_all_explored: world_config_bool(world_configs, "CONFIG_START_ALL_EXPLORED", false),
        start_all_reputation: world_config_bool(world_configs, "CONFIG_START_ALL_REP", false),
    })
}

pub(super) fn build_player_rest_rates(
    world_configs: &wow_config::WorldConfigSet,
) -> Arc<wow_world::session::PlayerRestRatePolicyLikeCpp> {
    Arc::new(wow_world::session::PlayerRestRatePolicyLikeCpp {
        offline_wilderness: world_config_f32(world_configs, "RATE_REST_OFFLINE_IN_WILDERNESS", 1.0),
        offline_tavern_or_city: world_config_f32(
            world_configs,
            "RATE_REST_OFFLINE_IN_TAVERN_OR_CITY",
            1.0,
        ),
        ingame: world_config_f32(world_configs, "RATE_REST_INGAME", 1.0),
    })
}

pub(super) fn build_progression(
    player_xp_table: &Arc<Vec<u32>>,
    exploration_base_xp_store: &Arc<wow_data::ExplorationBaseXpStoreLikeCpp>,
    world_configs: &wow_config::WorldConfigSet,
) -> Arc<wow_world::session::ProgressionCatalogsLikeCpp> {
    Arc::new(wow_world::session::ProgressionCatalogsLikeCpp {
        no_reset_talent_cost: world_config_bool(
            world_configs,
            "CONFIG_NO_RESET_TALENT_COST",
            false,
        ),
        player_xp: Arc::clone(player_xp_table),
        exploration_base_xp: Arc::clone(exploration_base_xp_store),
        exploration_xp_rate: world_config_f32(world_configs, "RATE_XP_EXPLORE", 1.0),
        min_discovered_scaled_xp_ratio: world_config_u32(
            world_configs,
            "CONFIG_MIN_DISCOVERED_SCALED_XP_RATIO",
            0,
        ),
    })
}

pub(super) fn build_player_regeneration_rates(
    world_configs: &wow_config::WorldConfigSet,
) -> Arc<wow_world::PlayerRegenerationRatesLikeCpp> {
    Arc::new(player_regeneration_rates_like_cpp(world_configs))
}
