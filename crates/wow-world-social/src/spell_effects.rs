// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The represented reputation and force-deselect spell effects (#1363 slice
//! 12b3, the 12b5 pair), moved from the World `session/spell_effects`: the
//! reputation change through the Player's `ReputationMgr`, and the
//! force-deselect evidence recorded into the social state.

use std::sync::Arc;

use wow_core::ObjectGuid;
use wow_world_core::session::ReputationGainSourceLikeCpp;

use crate::{SocialSessionHostLikeCpp, calculate_reputation_gain_like_cpp};

#[cfg(any(test, feature = "test-fixtures"))]
const DEFAULT_VISIBILITY_DISTANCE_YARDS_LIKE_CPP: u32 = 100;

/// C++ `Spell::EffectReputation`.
///
/// Represented boundary: current player target only. This follows the C++
/// spell-source reputation formula and `ReputationMgr::ModifyReputation`
/// packet path; generic player targets, persistence and script callbacks
/// remain outside this bounded slice.
pub fn apply_reputation_effect_like_cpp<H: SocialSessionHostLikeCpp>(
    host: &mut H,
    damage: i32,
    misc_value: i32,
    target_guid: ObjectGuid,
) -> bool {
    let Some(player_guid) = host.player_guid() else {
        return false;
    };
    if target_guid != player_guid {
        return false;
    }
    let Ok(faction_id) = u32::try_from(misc_value) else {
        return false;
    };
    let Some(faction_store) = host.hub_ref().catalogs.faction_store().map(Arc::clone) else {
        return false;
    };
    let Some(faction_entry) = faction_store.get(faction_id).cloned() else {
        return false;
    };

    let reputation = calculate_reputation_gain_like_cpp(
        host,
        ReputationGainSourceLikeCpp::Spell,
        0,
        damage,
        faction_id,
        false,
    );

    let reputation_spillover_template_store = host
        .hub_ref()
        .catalogs
        .reputation_spillover_template_store()
        .map(Arc::clone);
    let friendship_rep_reaction_store = host
        .hub_ref()
        .catalogs
        .friendship_rep_reaction_store()
        .map(Arc::clone);
    let paragon_reputation_store = host
        .hub_ref()
        .catalogs
        .paragon_reputation_store()
        .map(Arc::clone);
    let currency_types_store = host
        .hub_ref()
        .catalogs
        .currency_types_store()
        .map(Arc::clone);
    let db_spillover_template = reputation_spillover_template_store
        .as_deref()
        .and_then(|store| store.get(faction_id));
    let options = wow_progression::mgr::SetReputationOptionsLikeCpp {
        incremental: true,
        spillover_only: false,
        no_spillover: false,
        reputation_gain_rate: host.hub_ref().config.reputation_rates_like_cpp().gain,
        paragon_reward_quest_status_none_like_cpp: true,
        renown_current_level_like_cpp: 0,
        renown_currency_increased_cap_quantity_like_cpp: 0,
        player_race: host.hub_ref().player_race_like_cpp(),
        player_class: host.hub_ref().player_class_like_cpp(),
    };
    let Some((outcome, packet)) = host.hub_mut().mutate_reputation_mgr_like_cpp(|mgr| {
        let outcome = mgr.set_reputation_like_cpp(
            &faction_entry,
            reputation,
            options,
            &faction_store,
            db_spillover_template,
            friendship_rep_reaction_store.as_deref(),
            paragon_reputation_store.as_deref(),
            currency_types_store.as_deref(),
        );
        let packet = outcome
            .send_state_rep_list_id
            .map(|rep_list_id| mgr.set_faction_standing_packet_like_cpp(Some(rep_list_id)));
        (outcome, packet)
    }) else {
        return false;
    };
    if let Some(packet) = packet {
        host.send_packet(&packet);
    }
    outcome.applied
}

/// C++ `Spell::EffectForceDeselect`.
///
/// Represented boundary: packet construction and effect evidence only.
/// C++ delivers `SMSG_BREAK_TARGET` and `SMSG_CLEAR_TARGET` to hostile
/// visible clients around the caster, then stops attacker pets without
/// threat lists. The hostile visible-set fanout and attacker traversal are
/// deferred to the live visibility/combat runtime.
pub fn apply_force_deselect_effect_like_cpp<H: SocialSessionHostLikeCpp>(host: &mut H) -> bool {
    let Some(caster_guid) = host.player_guid() else {
        return false;
    };

    #[cfg(any(test, feature = "test-fixtures"))]
    if let Some(social) = host.social_limits_for_test_like_cpp() {
        use wow_packet::ServerPacket;
        let break_target_packet_bytes = wow_packet::packets::combat::BreakTarget {
            unit_guid: caster_guid,
        }
        .to_bytes();
        let clear_target_packet_bytes =
            wow_packet::packets::spell::ClearTarget { guid: caster_guid }.to_bytes();

        social.record_represented_force_deselect_for_test_like_cpp(
            crate::RepresentedForceDeselectLikeCpp {
                caster_guid,
                visibility_range_yards: DEFAULT_VISIBILITY_DISTANCE_YARDS_LIKE_CPP,
                break_target_packet_bytes,
                clear_target_packet_bytes,
                hostile_visible_fanout_unrepresented: true,
                attacker_pet_attack_stop_unrepresented: true,
            },
        );
    }
    #[cfg(not(any(test, feature = "test-fixtures")))]
    let _ = caster_guid;
    true
}
