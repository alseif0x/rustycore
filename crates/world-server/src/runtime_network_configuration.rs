//! Ordered network, MMap and creature-aggro configuration.

use super::*;

pub(super) struct RuntimeNetworkConfiguration {
    pub(super) realm_addr: SocketAddr,
    pub(super) instance_addr: SocketAddr,
    pub(super) instance_port: u16,
    pub(super) max_expansion: u8,
    pub(super) mmap_runtime_config: MMapRuntimeConfigLikeCpp,
    pub(super) mmap_pathfinder: Option<Arc<WorldMMapPathfinderWorkerLikeCpp>>,
    pub(super) creature_aggro_config: wow_world::session::CreatureAggroConfigLikeCpp,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build_runtime_network_configuration(
    world_configs: &wow_config::WorldConfigSet,
    mmap_disabled_map_ids: HashSet<u32>,
    map_store: &Arc<wow_data::MapStore>,
    data_dir: &str,
    locale: &str,
    faction_template_store: &Arc<wow_data::progression_rewards::FactionTemplateStore>,
    progression_faction_store: &Arc<wow_data::progression_rewards::FactionStore>,
    disable_mgr: &Arc<wow_data::DisableMgrLikeCpp>,
    spell_misc_store: &Arc<wow_data::SpellMiscStore>,
    spell_range_store: &Arc<wow_data::SpellRangeStore>,
    spell_duration_store: &Arc<wow_data::SpellDurationStore>,
    spell_cooldowns_store: &Arc<wow_data::SpellCooldownsStore>,
    spell_category_store: &Arc<wow_data::SpellCategoryStore>,
    spell_x_spell_visual_store: &Arc<wow_data::SpellXSpellVisualStore>,
    spell_target_restrictions_store: &Arc<wow_data::SpellTargetRestrictionsStore>,
    spell_casting_requirements_store: &Arc<wow_data::SpellCastingRequirementsStore>,
    spell_aura_restrictions_store: &Arc<wow_data::SpellAuraRestrictionsStore>,
    spell_store: &Arc<wow_data::SpellStore>,
    spell_threat_store: &Arc<wow_data::SpellThreatStoreLikeCpp>,
    spell_chain_store: &Arc<wow_data::SpellChainStoreLikeCpp>,
    spell_linked_store: &Arc<wow_data::SpellLinkedStoreLikeCpp>,
    condition_store: &Arc<wow_data::ConditionEntriesByTypeStore>,
    spell_script_exact_spell_ids: &Arc<BTreeSet<u32>>,
    spell_script_all_rank_root_spell_ids: &Arc<BTreeSet<u32>>,
    legacy_spell_script_spell_ids: &Arc<BTreeSet<u32>>,
    spell_linked_rejected_trigger_spell_ids: &Arc<BTreeSet<u32>>,
    spell_custom_attribute_store: &Arc<wow_data::SpellCustomAttributeStoreLikeCpp>,
    difficulty_store: &Arc<wow_data::DifficultyStore>,
    creature_template_lifecycle_store: &Arc<wow_data::CreatureTemplateLifecycleStoreLikeCpp>,
    chr_races_store: &Arc<wow_data::character_progression::ChrRacesStore>,
) -> Result<RuntimeNetworkConfiguration> {
    // Network configuration
    let bind_ip = wow_config::get_string_default("BindIP", "0.0.0.0");
    let world_port = world_config_u16(world_configs, "CONFIG_PORT_WORLD", 8085);
    let instance_port = world_config_u16(world_configs, "CONFIG_PORT_INSTANCE", 8086);
    let max_expansion = world_config_u8(world_configs, "CONFIG_EXPANSION", 2);
    let mmap_runtime_config = mmap_runtime_config_like_cpp(world_configs, mmap_disabled_map_ids);
    info!(
        "WORLD: MMap pathfinding: {}, data directory: {}/mmaps",
        if mmap_runtime_config.enabled {
            "enabled"
        } else {
            "disabled"
        },
        mmap_runtime_config.data_dir
    );
    let mmap_pathfinder = mmap_runtime_config.enabled.then(|| {
        Arc::new(
            WorldMMapPathfinderWorkerLikeCpp::spawn_with_parent_map_data_like_cpp(
                &mmap_runtime_config.data_dir,
                map_store.parent_child_map_data_like_cpp(),
            ),
        )
    });

    let realm_addr: SocketAddr = format!("{bind_ip}:{world_port}")
        .parse()
        .context("Invalid bind address")?;
    let instance_addr: SocketAddr = format!("{bind_ip}:{instance_port}")
        .parse()
        .context("Invalid instance bind address")?;

    info!("Starting realm listener on {realm_addr}");
    info!("Starting instance listener on {instance_addr}");
    let mut creature_aggro_config = creature_aggro_config_like_cpp(world_configs);
    creature_aggro_config.expected_stat_store = wow_data::ExpectedStatStore::load(data_dir, locale)
        .ok()
        .map(Arc::new);
    creature_aggro_config.faction_template_store = Some(Arc::clone(faction_template_store));
    creature_aggro_config.faction_store = Some(Arc::clone(progression_faction_store));
    creature_aggro_config.map_store = Some(Arc::clone(map_store));
    creature_aggro_config.disable_mgr = Some(Arc::clone(disable_mgr));
    creature_aggro_config.spell_misc_store = Some(Arc::clone(spell_misc_store));
    creature_aggro_config.spell_range_store = Some(Arc::clone(spell_range_store));
    creature_aggro_config.spell_duration_store = Some(Arc::clone(spell_duration_store));
    creature_aggro_config.spell_cooldowns_store = Some(Arc::clone(spell_cooldowns_store));
    creature_aggro_config.spell_category_store = Some(Arc::clone(spell_category_store));
    creature_aggro_config.spell_x_spell_visual_store = Some(Arc::clone(spell_x_spell_visual_store));
    creature_aggro_config.spell_target_restrictions_store =
        Some(Arc::clone(spell_target_restrictions_store));
    creature_aggro_config.spell_casting_requirements_store =
        Some(Arc::clone(spell_casting_requirements_store));
    creature_aggro_config.spell_aura_restrictions_store =
        Some(Arc::clone(spell_aura_restrictions_store));
    creature_aggro_config.spell_store = Some(Arc::clone(spell_store));
    creature_aggro_config.spell_threat_store = Some(Arc::clone(spell_threat_store));
    creature_aggro_config.spell_chain_store = Some(Arc::clone(spell_chain_store));
    creature_aggro_config.spell_linked_store = Some(Arc::clone(spell_linked_store));
    creature_aggro_config.spell_condition_store = Some(Arc::clone(condition_store));
    creature_aggro_config.spell_script_exact_spell_ids_like_cpp =
        Some(Arc::clone(spell_script_exact_spell_ids));
    creature_aggro_config.spell_script_all_rank_root_spell_ids_like_cpp =
        Some(Arc::clone(spell_script_all_rank_root_spell_ids));
    creature_aggro_config.legacy_spell_script_spell_ids_like_cpp =
        Some(Arc::clone(legacy_spell_script_spell_ids));
    creature_aggro_config.spell_linked_rejected_trigger_spell_ids_like_cpp =
        Some(Arc::clone(spell_linked_rejected_trigger_spell_ids));
    creature_aggro_config.spell_custom_attribute_store =
        Some(Arc::clone(spell_custom_attribute_store));
    creature_aggro_config.difficulty_store = Some(Arc::clone(difficulty_store));
    creature_aggro_config.creature_template_lifecycle_store =
        Some(Arc::clone(creature_template_lifecycle_store));
    creature_aggro_config.chr_races_store = Some(Arc::clone(chr_races_store));

    Ok(RuntimeNetworkConfiguration {
        realm_addr,
        instance_addr,
        instance_port,
        max_expansion,
        mmap_runtime_config,
        mmap_pathfinder,
        creature_aggro_config,
    })
}
