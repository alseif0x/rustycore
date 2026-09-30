//! Ordered network configuration composition.

use crate::{
    legacy_creature_aggro_config_like_cpp, mmap_runtime_config_like_cpp, world_config_u8,
    world_config_u16,
};
use anyhow::Context;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;
use wow_world::WorldMMapPathfinderWorkerLikeCpp;

pub(super) struct NetworkConfiguration {
    pub(super) legacy_creature_aggro_config: wow_world::session::LegacyCreatureAggroConfigLikeCpp,
    pub(super) instance_addr: std::net::SocketAddr,
    pub(super) realm_addr: std::net::SocketAddr,
    pub(super) mmap_pathfinder: Option<Arc<wow_world::WorldMMapPathfinderWorkerLikeCpp>>,
    pub(super) mmap_runtime_config: wow_world::MMapRuntimeConfigLikeCpp,
    pub(super) max_expansion: u8,
    pub(super) instance_port: u16,
    pub(super) world_port: u16,
    pub(super) bind_ip: String,
}

pub(super) fn load(
    world_configs: &wow_config::WorldConfigSet,
    data_dir: &str,
    locale: &str,
    mmap_disabled_map_ids: std::collections::HashSet<u32>,
    player: &crate::session_resources::SessionPlayerCatalogCapabilitiesLikeCpp,
    spells: &crate::session_resources::SessionSpellCatalogCapabilitiesLikeCpp,
    world: &crate::session_resources::SessionWorldCatalogCapabilitiesLikeCpp,
    progression: &crate::session_resources::SessionProgressionCapabilitiesLikeCpp,
    spell_info: &super::spell_info_startup::SpellInfoStartup,
    spell_cooldowns_store: &Arc<wow_data::SpellCooldownsStore>,
    jump_charge: &super::jump_charge_startup::JumpChargeCatalogs,
) -> anyhow::Result<NetworkConfiguration> {
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
                world.map_store.parent_child_map_data_like_cpp(),
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
    let mut legacy_creature_aggro_config = legacy_creature_aggro_config_like_cpp(world_configs);
    legacy_creature_aggro_config.expected_stat_store =
        wow_data::ExpectedStatStore::load(data_dir, locale)
            .ok()
            .map(Arc::new);
    legacy_creature_aggro_config.faction_template_store =
        Some(Arc::clone(&progression.faction_template_store));
    legacy_creature_aggro_config.faction_store =
        Some(Arc::clone(&progression.progression_faction_store));
    legacy_creature_aggro_config.map_store = Some(Arc::clone(&world.map_store));
    legacy_creature_aggro_config.disable_mgr = Some(Arc::clone(&player.disable_mgr));
    legacy_creature_aggro_config.spell_misc_store = Some(Arc::clone(&spell_info.spell_misc_store));
    legacy_creature_aggro_config.spell_range_store = Some(Arc::clone(&spells.spell_range_store));
    legacy_creature_aggro_config.spell_duration_store =
        Some(Arc::clone(&spells.spell_duration_store));
    legacy_creature_aggro_config.spell_cooldowns_store = Some(Arc::clone(spell_cooldowns_store));
    legacy_creature_aggro_config.spell_category_store =
        Some(Arc::clone(&spell_info.spell_category_store));
    legacy_creature_aggro_config.spell_x_spell_visual_store =
        Some(Arc::clone(&jump_charge.spell_x_spell_visual_store));
    legacy_creature_aggro_config.spell_target_restrictions_store =
        Some(Arc::clone(&spell_info.spell_target_restrictions_store));
    legacy_creature_aggro_config.spell_casting_requirements_store =
        Some(Arc::clone(&spell_info.spell_casting_requirements_store));
    legacy_creature_aggro_config.spell_aura_restrictions_store =
        Some(Arc::clone(&spell_info.spell_aura_restrictions_store));
    legacy_creature_aggro_config.spell_store = Some(Arc::clone(&spells.spell_store));
    legacy_creature_aggro_config.spell_threat_store = Some(Arc::clone(&spells.spell_threat_store));
    legacy_creature_aggro_config.spell_chain_store = Some(Arc::clone(&spells.spell_chain_store));
    legacy_creature_aggro_config.spell_linked_store = Some(Arc::clone(&spells.spell_linked_store));
    legacy_creature_aggro_config.spell_condition_store = Some(Arc::clone(&player.condition_store));
    legacy_creature_aggro_config.spell_script_exact_spell_ids_like_cpp =
        Some(Arc::clone(&spells.spell_script_exact_spell_ids));
    legacy_creature_aggro_config.spell_script_all_rank_root_spell_ids_like_cpp =
        Some(Arc::clone(&spells.spell_script_all_rank_root_spell_ids));
    legacy_creature_aggro_config.legacy_spell_script_spell_ids_like_cpp =
        Some(Arc::clone(&spells.legacy_spell_script_spell_ids));
    legacy_creature_aggro_config.spell_linked_rejected_trigger_spell_ids_like_cpp =
        Some(Arc::clone(&spells.spell_linked_rejected_trigger_spell_ids));
    legacy_creature_aggro_config.spell_custom_attribute_store =
        Some(Arc::clone(&spells.spell_custom_attribute_store));
    legacy_creature_aggro_config.difficulty_store = Some(Arc::clone(&player.difficulty_store));
    legacy_creature_aggro_config.creature_template_lifecycle_store =
        Some(Arc::clone(&world.creature_template_lifecycle_store));
    legacy_creature_aggro_config.chr_races_store = Some(Arc::clone(&player.chr_races_store));
    Ok(NetworkConfiguration {
        bind_ip,
        world_port,
        instance_port,
        max_expansion,
        mmap_runtime_config,
        mmap_pathfinder,
        realm_addr,
        instance_addr,
        legacy_creature_aggro_config,
    })
}

pub(super) fn build_listener_policy(
    world_configs: &wow_config::WorldConfigSet,
    ip_location_store: &Arc<wow_core::IpLocationStore>,
) -> wow_network::WorldListenerPolicyLikeCpp {
    let world_listener_policy = wow_network::WorldListenerPolicyLikeCpp {
        max_overspeed_pings: crate::world_config_u32(
            world_configs,
            "CONFIG_MAX_OVERSPEED_PINGS",
            2,
        ),
        socket_timeouts: wow_network::SocketTimeoutsLikeCpp {
            unauthenticated_secs: u64::from(crate::world_config_u32(
                world_configs,
                "CONFIG_SOCKET_TIMEOUTTIME",
                900,
            )),
            active_secs: u64::from(crate::world_config_u32(
                world_configs,
                "CONFIG_SOCKET_TIMEOUTTIME_ACTIVE",
                60,
            )),
        },
        ip_location_store: Some(Arc::clone(ip_location_store)),
    };
    world_listener_policy
}
