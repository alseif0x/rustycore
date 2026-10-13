// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The represented wild summon-object effect (#1363 slice 9c), from the World
//! `session/spell_effects/effect_summon.rs`; the slot effect stays there.

use super::SpellEffectHostLikeCpp;
use super::summon_contracts::*;
use crate::RepresentedSpellFocusObjectLikeCpp;
use wow_core::Position;
use wow_data::spell_duration_ms_like_cpp;
use wow_packet::packets::spell::SpellTargetData;

/// Dormant live resolver for C++ `Spell::EffectSummonObjectWild`.
///
/// C++ anchors: `SpellEffects.cpp:2937-2986` and
/// `GameObject.cpp:1187-1200`. This resolves the DB-backed template,
/// spell destination or caster close-point fallback, and spell duration,
/// then delegates the map-owned body to `wow_map`.
#[allow(dead_code)]
pub fn apply_effect_summon_object_wild_like_cpp<H: SpellEffectHostLikeCpp>(
    host: &mut H,
    spell_id: i32,
    effect: &wow_data::SpellEffectInfo,
    target_data: &SpellTargetData,
) -> Option<ApplyEffectSummonObjectWildSessionOutcomeLikeCpp> {
    apply_effect_summon_object_wild_with_focus_like_cpp(host, spell_id, effect, target_data, None)
}

pub fn apply_effect_summon_object_wild_with_focus_like_cpp<H: SpellEffectHostLikeCpp>(
    host: &mut H,
    spell_id: i32,
    effect: &wow_data::SpellEffectInfo,
    target_data: &SpellTargetData,
    focus_object: Option<RepresentedSpellFocusObjectLikeCpp>,
) -> Option<ApplyEffectSummonObjectWildSessionOutcomeLikeCpp> {
    if effect.effect != wow_data::spell::spell_effect_types::SPELL_EFFECT_SUMMON_OBJECT_WILD {
        return None;
    }
    let Ok(template_entry) = u32::try_from(effect.effect_misc_value_1) else {
        return Some(ApplyEffectSummonObjectWildSessionOutcomeLikeCpp {
            status: ApplyEffectSummonObjectWildSessionStatusLikeCpp::InvalidTemplateEntry,
            template_entry: None,
            duration_ms: None,
            explicit_destination_used: false,
            close_point_fallback_represented: false,
            map_outcome: None,
        });
    };
    let Some(template_store) = host
        .hub_ref()
        .catalogs
        .gameobject_template_lifecycle_store_like_cpp
        .as_deref()
    else {
        return Some(ApplyEffectSummonObjectWildSessionOutcomeLikeCpp {
            status: ApplyEffectSummonObjectWildSessionStatusLikeCpp::MissingTemplateStore,
            template_entry: Some(template_entry),
            duration_ms: None,
            explicit_destination_used: false,
            close_point_fallback_represented: false,
            map_outcome: None,
        });
    };
    let Some(template) = template_store.get(template_entry) else {
        return Some(ApplyEffectSummonObjectWildSessionOutcomeLikeCpp {
            status: ApplyEffectSummonObjectWildSessionStatusLikeCpp::MissingTemplate,
            template_entry: Some(template_entry),
            duration_ms: None,
            explicit_destination_used: false,
            close_point_fallback_represented: false,
            map_outcome: None,
        });
    };
    let Some(caster_guid) = host.player_guid() else {
        return Some(ApplyEffectSummonObjectWildSessionOutcomeLikeCpp {
            status: ApplyEffectSummonObjectWildSessionStatusLikeCpp::MissingCaster,
            template_entry: Some(template_entry),
            duration_ms: None,
            explicit_destination_used: true,
            close_point_fallback_represented: false,
            map_outcome: None,
        });
    };
    let position_outcome = if let Some(dest) = target_data.dst_location {
        wow_map::map::spell_effect_summon_object_wild_position_like_cpp(
            Position::ZERO,
            0.0,
            0.0,
            Some(dest.position),
        )
    } else {
        let Some(caster_position) = host.hub_ref().player_position_like_cpp() else {
            return Some(ApplyEffectSummonObjectWildSessionOutcomeLikeCpp {
                status: ApplyEffectSummonObjectWildSessionStatusLikeCpp::MissingCasterPosition,
                template_entry: Some(template_entry),
                duration_ms: None,
                explicit_destination_used: false,
                close_point_fallback_represented: false,
                map_outcome: None,
            });
        };
        let close_point_source_position = focus_object
            .map(|focus| focus.position)
            .unwrap_or(caster_position);
        let represented_source_reach = if focus_object.is_some() {
            // C++ calls `target->GetClosePoint`; represented GameObject size
            // is not carried here yet, so focus-backed fallback uses the
            // focus position with no extra object reach instead of the
            // caster combat reach.
            0.0
        } else {
            host.core()
                .canonical_player_combat_reach_snapshot_like_cpp()
        };
        wow_map::map::spell_effect_summon_object_wild_position_like_cpp(
            close_point_source_position,
            represented_source_reach,
            close_point_source_position.orientation,
            None,
        )
    };
    let Some(manager) = host.core().canonical_map_manager.as_ref().cloned() else {
        return Some(ApplyEffectSummonObjectWildSessionOutcomeLikeCpp {
            status: ApplyEffectSummonObjectWildSessionStatusLikeCpp::MissingCanonicalMapManager,
            template_entry: Some(template_entry),
            duration_ms: None,
            explicit_destination_used: position_outcome.explicit_destination_used,
            close_point_fallback_represented: position_outcome.close_point_fallback_used,
            map_outcome: None,
        });
    };
    let Some(player_map_key) = focus_object
        .map(|focus| focus.map_key)
        .or_else(|| host.core().current_canonical_player_map_key_like_cpp())
    else {
        return Some(ApplyEffectSummonObjectWildSessionOutcomeLikeCpp {
            status: ApplyEffectSummonObjectWildSessionStatusLikeCpp::MissingCanonicalPlayerMap,
            template_entry: Some(template_entry),
            duration_ms: None,
            explicit_destination_used: position_outcome.explicit_destination_used,
            close_point_fallback_represented: position_outcome.close_point_fallback_used,
            map_outcome: None,
        });
    };
    let spell_id_u32 = u32::try_from(spell_id).unwrap_or(0);
    let duration_index = host
        .hub_ref()
        .catalogs
        .spell_catalogs
        .spell_misc_store
        .as_deref()
        .and_then(|store| store.get_by_spell_id(spell_id_u32))
        .map(|entry| u32::from(entry.duration_index))
        .unwrap_or(0);
    let duration_ms = spell_duration_ms_like_cpp(
        duration_index,
        host.hub_ref()
            .catalogs
            .spell_catalogs
            .spell_duration_store
            .as_deref(),
    );
    let lifecycle_record = wow_data::gameobject_template_lifecycle_record_like_cpp(template);
    let Ok(mut manager) = manager.lock() else {
        return Some(ApplyEffectSummonObjectWildSessionOutcomeLikeCpp {
            status: ApplyEffectSummonObjectWildSessionStatusLikeCpp::MissingCanonicalMapManager,
            template_entry: Some(template_entry),
            duration_ms: Some(duration_ms),
            explicit_destination_used: position_outcome.explicit_destination_used,
            close_point_fallback_represented: position_outcome.close_point_fallback_used,
            map_outcome: None,
        });
    };
    let Some(managed) = manager.find_map_mut(player_map_key.map_id, player_map_key.instance_id)
    else {
        return Some(ApplyEffectSummonObjectWildSessionOutcomeLikeCpp {
            status: ApplyEffectSummonObjectWildSessionStatusLikeCpp::MissingManagedMap,
            template_entry: Some(template_entry),
            duration_ms: Some(duration_ms),
            explicit_destination_used: position_outcome.explicit_destination_used,
            close_point_fallback_represented: position_outcome.close_point_fallback_used,
            map_outcome: None,
        });
    };
    let map_outcome = managed.map_mut().spell_effect_summon_object_wild_like_cpp(
        caster_guid,
        spell_id_u32,
        lifecycle_record,
        position_outcome.position,
        duration_ms,
    );

    Some(ApplyEffectSummonObjectWildSessionOutcomeLikeCpp {
        status: ApplyEffectSummonObjectWildSessionStatusLikeCpp::MapResolved,
        template_entry: Some(template_entry),
        duration_ms: Some(duration_ms),
        explicit_destination_used: position_outcome.explicit_destination_used,
        close_point_fallback_represented: position_outcome.close_point_fallback_used,
        map_outcome: Some(map_outcome),
    })
}
