//! Creature AI state machines regression scenarios.
//!
//! Separated from the lib.rs root under #658.

use super::*;
use std::collections::HashMap;
use wow_instances::BossAiLikeCpp;

fn selector_input() -> CreatureAiSelectionInputLikeCpp {
    CreatureAiSelectionInputLikeCpp::default()
}

fn assert_close_like_cpp(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= f32::EPSILON,
        "expected {expected}, got {actual}"
    );
}

fn zone_player(
    low: i64,
    controlled_unit_lows: &[i64],
    vehicle_base_low: Option<i64>,
) -> CreatureZoneInCombatPlayerLikeCpp {
    CreatureZoneInCombatPlayerLikeCpp {
        player_guid: guid(low),
        is_alive: true,
        can_begin_combat: true,
        controlled_unit_guids: controlled_unit_lows.iter().copied().map(guid).collect(),
        vehicle_base_guid: vehicle_base_low.map(guid),
    }
}

fn target_candidate(
    low: i64,
    threat_order: u32,
    distance_to_me: f32,
) -> UnitAiTargetCandidateLikeCpp {
    UnitAiTargetCandidateLikeCpp {
        guid: guid(low),
        is_offline: false,
        is_current_victim: false,
        is_last_victim: false,
        threat_order,
        distance_to_me,
        is_player: true,
        has_aura: false,
    }
}

fn creature_with_boss_id(boss_id: Option<u32>) -> CreatureAI {
    CreatureAI::new(
        ObjectGuid::EMPTY,
        1,
        Position::ZERO,
        100,
        1,
        1,
        2,
        0.0,
        1,
        35,
        0,
        0,
        0,
        0,
        0,
        boss_id,
        0,
    )
}

fn guid(low: i64) -> ObjectGuid {
    ObjectGuid::new(0, low)
}

fn creature_view(guid: ObjectGuid, entry: u32, ai_enabled: bool) -> SummonListCreatureViewLikeCpp {
    SummonListCreatureViewLikeCpp {
        guid,
        entry,
        ai_enabled,
    }
}

fn resolve_from(
    creatures: &HashMap<ObjectGuid, SummonListCreatureViewLikeCpp>,
    guid: ObjectGuid,
) -> Option<SummonListCreatureViewLikeCpp> {
    creatures.get(&guid).copied()
}

mod scenarios_1;
mod scenarios_2;
