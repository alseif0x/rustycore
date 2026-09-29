use crate::spawn::{SpawnGroupFlags, SpawnObjectType};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DynamicRespawnScalingConfig {
    pub creature_rate: f64,
    pub creature_minimum_secs: u32,
    pub gameobject_rate: f64,
    pub gameobject_minimum_secs: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DynamicRespawnScalingNoopReason {
    DynamicModeDisabled,
    UnsupportedMode,
    BattlegroundOrArena,
    UnsupportedSpawnType,
    MissingSpawnMetadata,
    MissingDynamicSpawnRateFlag,
    MissingZonePlayerCount,
    ZeroZonePlayers,
    AdjustFactorAtLeastOne,
    DelayAtOrBelowMinimum,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DynamicRespawnScalingOutcome {
    pub delay_secs: u32,
    pub noop_reason: Option<DynamicRespawnScalingNoopReason>,
}

impl DynamicRespawnScalingOutcome {
    pub const fn unchanged(delay_secs: u32, reason: DynamicRespawnScalingNoopReason) -> Self {
        Self {
            delay_secs,
            noop_reason: Some(reason),
        }
    }

    pub const fn scaled(delay_secs: u32) -> Self {
        Self {
            delay_secs,
            noop_reason: None,
        }
    }

    pub const fn was_scaled(self) -> bool {
        self.noop_reason.is_none()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DynamicRespawnScalingContext {
    pub mode: u32,
    pub spawn_type: Option<SpawnObjectType>,
    pub spawn_metadata_present: bool,
    pub spawn_group_flags: Option<SpawnGroupFlags>,
    pub is_battleground_or_arena: bool,
    pub zone_player_count: Option<u32>,
    pub config: DynamicRespawnScalingConfig,
}

/// Rust equivalent of C++ `Map::ApplyDynamicModeRespawnScaling`.
///
/// C++ anchors:
/// - `GameObject.cpp:1665-1672` calls this before persisting GO respawn time.
/// - `Map.cpp:2242-2284` contains the dynamic respawn guards and formula.
/// - `Map.h:657-660` declares the map helper.
///
/// This helper is pure because RustyCore does not yet own the canonical map
/// spawn-metadata and zone-player-count stores needed by a `Map` method. Future
/// GameObject runtime wiring must pass canonical metadata/counts into this
/// function; this function must not read or mutate session-local fallback state.
pub fn apply_dynamic_mode_respawn_scaling_like_cpp(
    respawn_delay_secs: u32,
    context: DynamicRespawnScalingContext,
) -> DynamicRespawnScalingOutcome {
    if context.mode == 0 {
        return DynamicRespawnScalingOutcome::unchanged(
            respawn_delay_secs,
            DynamicRespawnScalingNoopReason::DynamicModeDisabled,
        );
    }

    if context.mode != 1 {
        return DynamicRespawnScalingOutcome::unchanged(
            respawn_delay_secs,
            DynamicRespawnScalingNoopReason::UnsupportedMode,
        );
    }

    if context.is_battleground_or_arena {
        return DynamicRespawnScalingOutcome::unchanged(
            respawn_delay_secs,
            DynamicRespawnScalingNoopReason::BattlegroundOrArena,
        );
    }

    let Some(spawn_type) = context.spawn_type else {
        return DynamicRespawnScalingOutcome::unchanged(
            respawn_delay_secs,
            DynamicRespawnScalingNoopReason::UnsupportedSpawnType,
        );
    };

    if !matches!(
        spawn_type,
        SpawnObjectType::Creature | SpawnObjectType::GameObject
    ) {
        return DynamicRespawnScalingOutcome::unchanged(
            respawn_delay_secs,
            DynamicRespawnScalingNoopReason::UnsupportedSpawnType,
        );
    }

    if !context.spawn_metadata_present {
        return DynamicRespawnScalingOutcome::unchanged(
            respawn_delay_secs,
            DynamicRespawnScalingNoopReason::MissingSpawnMetadata,
        );
    }

    let Some(spawn_group_flags) = context.spawn_group_flags else {
        return DynamicRespawnScalingOutcome::unchanged(
            respawn_delay_secs,
            DynamicRespawnScalingNoopReason::MissingSpawnMetadata,
        );
    };

    if !spawn_group_flags.contains(SpawnGroupFlags::DYNAMIC_SPAWN_RATE) {
        return DynamicRespawnScalingOutcome::unchanged(
            respawn_delay_secs,
            DynamicRespawnScalingNoopReason::MissingDynamicSpawnRateFlag,
        );
    }

    let Some(player_count) = context.zone_player_count else {
        return DynamicRespawnScalingOutcome::unchanged(
            respawn_delay_secs,
            DynamicRespawnScalingNoopReason::MissingZonePlayerCount,
        );
    };

    if player_count == 0 {
        return DynamicRespawnScalingOutcome::unchanged(
            respawn_delay_secs,
            DynamicRespawnScalingNoopReason::ZeroZonePlayers,
        );
    }

    let (rate, time_minimum) = match spawn_type {
        SpawnObjectType::Creature => (
            context.config.creature_rate,
            context.config.creature_minimum_secs,
        ),
        SpawnObjectType::GameObject => (
            context.config.gameobject_rate,
            context.config.gameobject_minimum_secs,
        ),
        SpawnObjectType::AreaTrigger => {
            return DynamicRespawnScalingOutcome::unchanged(
                respawn_delay_secs,
                DynamicRespawnScalingNoopReason::UnsupportedSpawnType,
            );
        }
    };

    let adjust_factor = rate / f64::from(player_count);
    if adjust_factor >= 1.0 {
        return DynamicRespawnScalingOutcome::unchanged(
            respawn_delay_secs,
            DynamicRespawnScalingNoopReason::AdjustFactorAtLeastOne,
        );
    }

    if respawn_delay_secs <= time_minimum {
        return DynamicRespawnScalingOutcome::unchanged(
            respawn_delay_secs,
            DynamicRespawnScalingNoopReason::DelayAtOrBelowMinimum,
        );
    }

    let scaled = (f64::from(respawn_delay_secs) * adjust_factor).ceil() as u32;
    DynamicRespawnScalingOutcome::scaled(scaled.max(time_minimum))
}
