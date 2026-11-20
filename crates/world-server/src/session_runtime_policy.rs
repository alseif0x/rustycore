//! Session runtime registries and immutable policy construction.

use super::*;

pub(super) fn build_session_runtime_policy(
    player_registry: &Arc<PlayerRegistry>,
    game_event_quest_complete_tx: flume::Sender<GameEventQuestCompleteCommandLikeCpp>,
    group_registry: &Arc<GroupRegistry>,
    pending_invites: &Arc<PendingInvites>,
    world_configs: &wow_config::WorldConfigSet,
) -> SessionRuntimePolicyCapabilitiesLikeCpp {
    SessionRuntimePolicyCapabilitiesLikeCpp {
        player_registry: Arc::clone(player_registry),
        game_event_quest_complete_tx: game_event_quest_complete_tx,
        group_registry: Arc::clone(group_registry),
        pending_invites: Arc::clone(pending_invites),
        loot_drop_rates: loot_drop_rates_like_cpp(world_configs),
        reputation_rates: reputation_rates_like_cpp(world_configs),
        repair_cost_rate: repair_cost_rate_like_cpp(world_configs),
        durability_loss_on_death_rate: durability_loss_on_death_rate_like_cpp(world_configs),
        death_policy: death_policy_like_cpp(world_configs),
        group_visibility_mode: world_configs
            .get_int("CONFIG_GROUP_VISIBILITY")
            .map(|value| value as i32)
            .unwrap_or(1),
        stats_limits: stats_limits_like_cpp(world_configs),
        reset_schedule: reset_schedule_like_cpp(world_configs),
        offhand_check_at_spell_unlearn: world_config_bool(
            world_configs,
            "CONFIG_OFFHAND_CHECK_AT_SPELL_UNLEARN",
            true,
        ),
        vmap_indoor_check: world_config_bool(world_configs, "CONFIG_VMAP_INDOOR_CHECK", false),
        cast_unstuck_enabled: world_config_bool(world_configs, "CONFIG_CAST_UNSTUCK", true),
        quest_low_level_hide_diff: world_config_u32(
            world_configs,
            "CONFIG_QUEST_LOW_LEVEL_HIDE_DIFF",
            4,
        ),
        quest_high_level_hide_diff: world_config_u32(
            world_configs,
            "CONFIG_QUEST_HIGH_LEVEL_HIDE_DIFF",
            7,
        ),
        quest_ignore_auto_complete: world_config_bool(
            world_configs,
            "CONFIG_QUEST_IGNORE_AUTO_COMPLETE",
            false,
        ),
        quest_ignore_raid: world_config_bool(world_configs, "CONFIG_QUEST_IGNORE_RAID", false),
        quest_enable_quest_tracker: world_config_bool(
            world_configs,
            "CONFIG_QUEST_ENABLE_QUEST_TRACKER",
            false,
        ),
        quest_status_keep_abandoned: quest_status_keep_abandoned_like_cpp(world_configs),
        enable_ae_loot: world_config_bool(world_configs, "CONFIG_ENABLE_AE_LOOT", false),
        server_expansion: world_config_u8(world_configs, "CONFIG_EXPANSION", 2),
        instance_ignore_raid: world_config_bool(
            world_configs,
            "CONFIG_INSTANCE_IGNORE_RAID",
            false,
        ),
        instance_ignore_level: world_config_bool(
            world_configs,
            "CONFIG_INSTANCE_IGNORE_LEVEL",
            false,
        ),
        max_instances_per_hour: world_config_u32(world_configs, "CONFIG_MAX_INSTANCES_PER_HOUR", 5),
        packet_spoof_config: PacketSpoofConfigLikeCpp {
            policy: world_config_u32(world_configs, "CONFIG_PACKET_SPOOF_POLICY", 1),
            ban_mode: world_config_u32(world_configs, "CONFIG_PACKET_SPOOF_BANMODE", 0),
            ban_duration_secs: world_config_u32(
                world_configs,
                "CONFIG_PACKET_SPOOF_BANDURATION",
                86_400,
            ),
        },
        player_save_interval_ms: world_config_u32(
            world_configs,
            "CONFIG_INTERVAL_SAVE",
            15 * 60 * 1000,
        ),
    }
}
