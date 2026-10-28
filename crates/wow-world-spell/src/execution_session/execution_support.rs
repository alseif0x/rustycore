// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Shared effect-route admission for represented spell execution.

use super::SpellExecutionHostLikeCpp;
use wow_core::ObjectGuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SpellEffectExecutionRouteLikeCpp {
    Direct,
    PostGo,
    ApplyAura,
    SourceInert,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SpellEffectRouteContextLikeCpp {
    pub player_guid: ObjectGuid,
    pub target_guid: ObjectGuid,
    pub selected_aura_constructor: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SelectedAuraExecutionRoutesLikeCpp {
    pub player_resurrection_sickness: bool,
    pub server_triggered_resurrection_sickness: bool,
    pub server_triggered_player_ghost_wisp: bool,
}

impl SelectedAuraExecutionRoutesLikeCpp {
    pub fn selected_effects(self) -> bool {
        self.player_resurrection_sickness || self.server_triggered_player_ghost_wisp
    }

    pub fn selected_full_server_aura(self) -> bool {
        self.server_triggered_resurrection_sickness || self.server_triggered_player_ghost_wisp
    }
}

pub(super) fn selected_aura_execution_routes_like_cpp<H: SpellExecutionHostLikeCpp>(
    host: &H,
    spell_id: i32,
    player_guid: ObjectGuid,
    target_guid: ObjectGuid,
    caster_guid: ObjectGuid,
    metadata: wow_entities::SpellCastMetadata,
) -> SelectedAuraExecutionRoutesLikeCpp {
    let player_resurrection_sickness = target_guid == player_guid
        && caster_guid == player_guid
        && super::resurrection_sickness::is_resurrection_sickness_spell(host, spell_id);
    let full_mask_trigger =
        metadata.trigger_cast_flags_like_cpp == wow_entities::TRIGGERED_FULL_MASK_LIKE_CPP;
    SelectedAuraExecutionRoutesLikeCpp {
        player_resurrection_sickness,
        server_triggered_resurrection_sickness: !metadata.from_client
            && full_mask_trigger
            && player_resurrection_sickness,
        server_triggered_player_ghost_wisp: super::resurrection_auras::is_selected_self_cast(
            host,
            spell_id,
            target_guid,
            caster_guid,
            metadata.from_client,
            full_mask_trigger,
        ),
    }
}

fn is_direct_spell_effect_like_cpp(effect: u32) -> bool {
    use wow_data::spell::spell_effect_types as effect_types;

    matches!(
        effect,
        effect_types::SPELL_EFFECT_INSTAKILL
            | super::resurrection_offer::SPELL_EFFECT_RESURRECT
            | super::resurrection_offer::SPELL_EFFECT_RESURRECT_NEW
            | super::resurrection_offer::SPELL_EFFECT_RESURRECT_WITH_AURA
            | effect_types::SPELL_EFFECT_HEAL
            | effect_types::SPELL_EFFECT_HEAL_MECHANICAL
            | effect_types::SPELL_EFFECT_HEAL_MAX_HEALTH
            | effect_types::SPELL_EFFECT_HEAL_PCT
            | effect_types::SPELL_EFFECT_ADD_EXTRA_ATTACKS
            | effect_types::SPELL_EFFECT_INEBRIATE
            | effect_types::SPELL_EFFECT_REPUTATION
            | effect_types::SPELL_EFFECT_POWER_DRAIN
            | effect_types::SPELL_EFFECT_ENERGIZE
            | effect_types::SPELL_EFFECT_ENERGIZE_PCT
            | effect_types::SPELL_EFFECT_POWER_BURN
            | effect_types::SPELL_EFFECT_HEALTH_LEECH
            | effect_types::SPELL_EFFECT_DUAL_WIELD
            | effect_types::SPELL_EFFECT_TITAN_GRIP
            | effect_types::SPELL_EFFECT_PARRY
            | effect_types::SPELL_EFFECT_BLOCK
            | effect_types::SPELL_EFFECT_PROFICIENCY
            | effect_types::SPELL_EFFECT_DURABILITY_DAMAGE
            | effect_types::SPELL_EFFECT_DURABILITY_DAMAGE_PCT
            | effect_types::SPELL_EFFECT_GIVE_HONOR
            | effect_types::SPELL_EFFECT_THREAT
            | effect_types::SPELL_EFFECT_DISTRACT
            | effect_types::SPELL_EFFECT_MODIFY_THREAT_PERCENT
            | effect_types::SPELL_EFFECT_ATTACK_ME
            | effect_types::SPELL_EFFECT_MODIFY_COOLDOWN
            | effect_types::SPELL_EFFECT_MODIFY_CHARGES
            | effect_types::SPELL_EFFECT_SANCTUARY
            | effect_types::SPELL_EFFECT_SELF_RESURRECT
            | effect_types::SPELL_EFFECT_STUCK
            | effect_types::SPELL_EFFECT_PLAY_MOVIE
            | effect_types::SPELL_EFFECT_DUEL
            | effect_types::SPELL_EFFECT_DISMISS_PET
            | effect_types::SPELL_EFFECT_FORCE_DESELECT
            | effect_types::SPELL_EFFECT_CHANGE_RAID_MARKER
            | effect_types::SPELL_EFFECT_LEARN_SPELL
            | effect_types::SPELL_EFFECT_LEARN_TRANSMOG_SET
            | effect_types::SPELL_EFFECT_UPGRADE_HEIRLOOM
            | effect_types::SPELL_EFFECT_UNCAGE_BATTLEPET
            | effect_types::SPELL_EFFECT_GRANT_BATTLEPET_LEVEL
            | effect_types::SPELL_EFFECT_GRANT_BATTLEPET_EXPERIENCE
            | effect_types::SPELL_EFFECT_CHANGE_BATTLEPET_QUALITY
            | effect_types::SPELL_EFFECT_PULL
            | effect_types::SPELL_EFFECT_LEARN_TRANSMOG_ILLUSION
            | effect_types::SPELL_EFFECT_TRADE_SKILL
            | effect_types::SPELL_EFFECT_QUEST_COMPLETE
            | effect_types::SPELL_EFFECT_KILL_CREDIT
            | effect_types::SPELL_EFFECT_KILL_CREDIT2
            | effect_types::SPELL_EFFECT_SCHOOL_DAMAGE
            | effect_types::SPELL_EFFECT_ENVIRONMENTAL_DAMAGE
    )
}

pub(super) fn is_explicitly_handled_player_aura_like_cpp(
    effect: &wow_data::SpellEffectInfo,
) -> bool {
    use wow_data::spell::aura_types as aura;

    effect.is_mounted_aura_like_cpp()
        || effect.is_provide_spell_focus_aura_like_cpp()
        || effect.is_battle_pet_xp_pct_aura_like_cpp()
        || matches!(
            effect.effect_aura,
            aura::SPELL_AURA_MOD_DETECT_RANGE
                | aura::SPELL_AURA_MOD_DETECTED_RANGE
                | aura::SPELL_AURA_MOD_RESTED_XP_CONSUMPTION
                | aura::SPELL_AURA_MOD_TAUNT
                | aura::SPELL_AURA_MOD_SCALE
                | aura::SPELL_AURA_MOD_SPEED_NO_CONTROL
                | aura::SPELL_AURA_MOD_INCREASE_SPEED
                | aura::SPELL_AURA_MOD_INCREASE_SWIM_SPEED
                | aura::SPELL_AURA_MOD_DECREASE_SPEED
                | aura::SPELL_AURA_USE_NORMAL_MOVEMENT_SPEED
                | aura::SPELL_AURA_MOD_MINIMUM_SPEED
                | aura::SPELL_AURA_MOD_MINIMUM_SPEED_RATE
                | aura::SPELL_AURA_MOD_SPEED_ALWAYS
                | aura::SPELL_AURA_MOD_SPEED_NOT_STACK
                | aura::SPELL_AURA_MOD_INCREASE_MOUNTED_SPEED
                | aura::SPELL_AURA_MOD_MOUNTED_SPEED_ALWAYS
                | aura::SPELL_AURA_MOD_MOUNTED_SPEED_NOT_STACK
                | aura::SPELL_AURA_MOD_INCREASE_MOUNTED_FLIGHT_SPEED
                | aura::SPELL_AURA_FLY
                | aura::SPELL_AURA_MOD_INCREASE_FLIGHT_SPEED
                | aura::SPELL_AURA_MOD_INCREASE_VEHICLE_FLIGHT_SPEED
                | aura::SPELL_AURA_MOD_MOUNTED_FLIGHT_SPEED_ALWAYS
                | aura::SPELL_AURA_MOD_FLIGHT_SPEED_NOT_STACK
        )
}

pub(super) fn classify_spell_effect_route_like_cpp(
    spell_info: &wow_data::SpellInfo,
    effect_type: u32,
    effect_index: Option<u32>,
    context: SpellEffectRouteContextLikeCpp,
) -> SpellEffectExecutionRouteLikeCpp {
    use wow_data::spell::spell_effect_types as effect_types;

    // Source NULL/Unused and the source-disabled handlers are legitimate
    // no-ops. Keep this ahead of every other route, and never classify DUMMY
    // as inert: its C++ path invokes scripts.
    if effect_types::is_cpp_null_or_unused_noop(effect_type) {
        return SpellEffectExecutionRouteLikeCpp::SourceInert;
    }
    if is_direct_spell_effect_like_cpp(effect_type) {
        return SpellEffectExecutionRouteLikeCpp::Direct;
    }
    if super::post_spell_go::is_post_spell_go_effect_type_like_cpp(effect_type) {
        return SpellEffectExecutionRouteLikeCpp::PostGo;
    }
    if effect_type != effect_types::SPELL_EFFECT_APPLY_AURA {
        return SpellEffectExecutionRouteLikeCpp::Unavailable;
    }

    if context.selected_aura_constructor
        || (context.target_guid != context.player_guid && context.target_guid.is_creature())
        || spell_info.effects().is_empty()
    {
        return SpellEffectExecutionRouteLikeCpp::ApplyAura;
    }

    let aura_row = effect_index.and_then(|index| {
        spell_info.effects().iter().find(|candidate| {
            candidate.effect_index == index
                && candidate.effect == effect_types::SPELL_EFFECT_APPLY_AURA
        })
    });
    if aura_row.is_some_and(is_explicitly_handled_player_aura_like_cpp) {
        return SpellEffectExecutionRouteLikeCpp::ApplyAura;
    }

    let apply_aura_rows = spell_info
        .effects()
        .iter()
        .filter(|candidate| candidate.effect == effect_types::SPELL_EFFECT_APPLY_AURA)
        .count();
    let generic_apply_aura_rows = spell_info
        .effects()
        .iter()
        .filter(|candidate| {
            candidate.effect == effect_types::SPELL_EFFECT_APPLY_AURA
                && !candidate.is_mounted_aura_like_cpp()
                && !candidate.is_provide_spell_focus_aura_like_cpp()
                && !candidate.is_battle_pet_xp_pct_aura_like_cpp()
                && candidate.effect_aura != wow_data::spell::aura_types::SPELL_AURA_MOD_DETECT_RANGE
                && candidate.effect_aura
                    != wow_data::spell::aura_types::SPELL_AURA_MOD_DETECTED_RANGE
                && candidate.effect_aura
                    != wow_data::spell::aura_types::SPELL_AURA_MOD_RESTED_XP_CONSUMPTION
                && candidate.effect_aura != wow_data::spell::aura_types::SPELL_AURA_MOD_TAUNT
        })
        .count();

    if generic_apply_aura_rows == 1 && apply_aura_rows == 1 {
        SpellEffectExecutionRouteLikeCpp::ApplyAura
    } else {
        SpellEffectExecutionRouteLikeCpp::Unavailable
    }
}

fn route_context(
    player_guid: ObjectGuid,
    target_guid: ObjectGuid,
    selected_aura_constructor: bool,
) -> SpellEffectRouteContextLikeCpp {
    SpellEffectRouteContextLikeCpp {
        player_guid,
        target_guid,
        selected_aura_constructor,
    }
}

pub(crate) fn validate_spell_effect_routes_like_cpp(
    spell_info: &wow_data::SpellInfo,
    primary_effect_type: u32,
    player_guid: ObjectGuid,
    target_guid: ObjectGuid,
    selected_aura_constructor: bool,
) -> Result<(), &'static str> {
    let context = route_context(player_guid, target_guid, selected_aura_constructor);
    if spell_info.effects().is_empty() {
        if primary_effect_type != 0
            && classify_spell_effect_route_like_cpp(spell_info, primary_effect_type, None, context)
                == SpellEffectExecutionRouteLikeCpp::Unavailable
        {
            return Err("normal cast contains an unimplemented spell effect");
        }
        return Ok(());
    }

    for effect in spell_info
        .effects()
        .iter()
        .filter(|effect| effect.effect != 0)
    {
        if classify_spell_effect_route_like_cpp(
            spell_info,
            effect.effect,
            Some(effect.effect_index),
            context,
        ) == SpellEffectExecutionRouteLikeCpp::Unavailable
        {
            return Err("normal cast contains an unimplemented spell effect");
        }
    }

    if primary_effect_type != 0 {
        let primary_index = spell_info
            .effects()
            .iter()
            .find(|effect| effect.effect == primary_effect_type)
            .map(|effect| effect.effect_index);
        if classify_spell_effect_route_like_cpp(
            spell_info,
            primary_effect_type,
            primary_index,
            context,
        ) == SpellEffectExecutionRouteLikeCpp::Unavailable
        {
            return Err("normal cast contains an unimplemented primary spell effect");
        }
    }
    Ok(())
}

pub fn validate_server_normal_spell_execution_availability_like_cpp<
    H: SpellExecutionHostLikeCpp,
>(
    host: &H,
    spell_info: &wow_data::SpellInfo,
    target_guid: ObjectGuid,
    metadata: wow_entities::SpellCastMetadata,
) -> Result<(), &'static str> {
    let player_guid = host.player_guid().ok_or("No player GUID")?;
    let caster_guid = metadata.caster_guid_override.unwrap_or(player_guid);
    let selected_routes = selected_aura_execution_routes_like_cpp(
        host,
        spell_info.spell_id,
        player_guid,
        target_guid,
        caster_guid,
        metadata,
    );
    validate_spell_effect_routes_like_cpp(
        spell_info,
        spell_info.effect_type,
        player_guid,
        target_guid,
        selected_routes.selected_effects(),
    )
}

#[cfg(test)]
#[path = "../../unit_tests/execution_session/execution_support.rs"]
mod tests;
