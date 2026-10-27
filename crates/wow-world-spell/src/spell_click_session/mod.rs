// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The represented spell-click clickee caster and its creature school
//! damage at the session boundary (#1363 slice 9b), moved from the World
//! `session/spell_state/cast.rs` and `session/world_entities/spell_click.rs`.
//!
//! C++ `WorldSession::HandleSpellClick` → `Creature::HandleSpellClick` casts
//! with the clickee as caster; the represented subset is the school-damage
//! cast. The bodies run over [`SpellClickHostLikeCpp`]; each host method is
//! the same-named `WorldSession` operation, called at the exact point the
//! body called it before the move.

use tracing::debug;
use wow_core::ObjectGuid;
use wow_world_entities::spell_click_values::{
    RepresentedSpellClickCastLikeCpp, RepresentedSpellClickClickeeCasterOutcomeLikeCpp,
    RepresentedSpellClickUnitRefLikeCpp, represented_spell_click_school_damage_amount_like_cpp,
};

use crate::AuraHostLikeCpp;

mod plan;

pub use plan::*;

/// The World session as the spell-click caster's host (supertrait: the aura host).
pub trait SpellClickHostLikeCpp: AuraHostLikeCpp {
    fn sync_player_registry_state_like_cpp(&self);
    fn represented_unit_values_update_to_update_object_like_cpp(
        &self,
        unit_guid: ObjectGuid,
        map_id: u16,
        values_update: &wow_entities::UnitValuesUpdate,
    ) -> Option<wow_packet::packets::update::UpdateObject>;
}

pub async fn execute_represented_spell_click_clickee_caster_like_cpp<H: SpellClickHostLikeCpp>(
    host: &mut H,
    creature_guid: ObjectGuid,
    cast: &RepresentedSpellClickCastLikeCpp,
    player_guid: ObjectGuid,
    clickee_owner_guid: Option<ObjectGuid>,
) -> RepresentedSpellClickClickeeCasterOutcomeLikeCpp {
    use wow_packet::packets::spell::{SpellCastVisual, SpellGoPkt, SpellTargetData};

    if cast.caster != RepresentedSpellClickUnitRefLikeCpp::Clickee {
        return RepresentedSpellClickClickeeCasterOutcomeLikeCpp::UnsupportedCaster;
    }
    let target_guid = match cast.target {
        RepresentedSpellClickUnitRefLikeCpp::Clicker => player_guid,
        RepresentedSpellClickUnitRefLikeCpp::Clickee => creature_guid,
        RepresentedSpellClickUnitRefLikeCpp::Owner => {
            return RepresentedSpellClickClickeeCasterOutcomeLikeCpp::UnsupportedTarget;
        }
    };
    match cast.original_caster {
        RepresentedSpellClickUnitRefLikeCpp::Clicker => {}
        RepresentedSpellClickUnitRefLikeCpp::Owner if clickee_owner_guid == Some(player_guid) => {}
        _ => {
            return RepresentedSpellClickClickeeCasterOutcomeLikeCpp::UnsupportedOriginalCaster;
        }
    }

    let Ok(spell_id) = i32::try_from(cast.spell_id) else {
        return RepresentedSpellClickClickeeCasterOutcomeLikeCpp::Failed;
    };
    let Some(spell_info) = host
        .hub_ref()
        .catalogs
        .spell_store()
        .and_then(|store| store.get(spell_id))
        .cloned()
    else {
        return RepresentedSpellClickClickeeCasterOutcomeLikeCpp::Failed;
    };
    let Some(damage_amount) = represented_spell_click_school_damage_amount_like_cpp(&spell_info)
    else {
        return RepresentedSpellClickClickeeCasterOutcomeLikeCpp::UnsupportedCaster;
    };

    let Some(cast_id) = host.next_represented_spell_cast_guid_like_cpp(spell_id) else {
        return RepresentedSpellClickClickeeCasterOutcomeLikeCpp::Failed;
    };
    host.send_packet(&SpellGoPkt {
        cast_data: Default::default(),
        caster: creature_guid,
        cast_id,
        original_cast_id: cast_id,
        spell_id,
        visual: SpellCastVisual::default(),
        cast_flags: 0,
        cast_flags_ex: 0,
        cast_time_ms: wow_world_core::session::game_time_ms_like_cpp(),
        target: SpellTargetData {
            flags: 0x2,
            unit: target_guid,
            item: ObjectGuid::EMPTY,
            ..SpellTargetData::default()
        },
        hit_targets: vec![target_guid],
        miss_targets: Vec::new(),
    });

    let result = if target_guid == player_guid {
        apply_represented_spell_click_creature_damage_to_clicker_like_cpp(
            host,
            spell_id,
            player_guid,
            damage_amount,
        )
        .await
    } else {
        apply_represented_spell_click_creature_damage_to_clickee_like_cpp(
            host,
            spell_id,
            creature_guid,
            damage_amount,
        )
        .await
    };
    match result {
        Ok(()) => RepresentedSpellClickClickeeCasterOutcomeLikeCpp::Executed,
        Err(_) => RepresentedSpellClickClickeeCasterOutcomeLikeCpp::Failed,
    }
}

pub(crate) async fn apply_represented_spell_click_creature_damage_to_clicker_like_cpp<
    H: SpellClickHostLikeCpp,
>(
    host: &mut H,
    spell_id: i32,
    player_guid: ObjectGuid,
    damage_amount: u32,
) -> Result<(), &'static str> {
    if host.player_guid() != Some(player_guid) {
        return Err("Target player not current session");
    }
    let Some((original_health, health_after, _, applied_damage, _)) = host
        .hub_mut()
        .apply_owned_player_damage_like_cpp(damage_amount, wow_constants::DeathState::JustDied)
    else {
        return Err("Target player owner not available");
    };
    if damage_amount > 0 && applied_damage == 0 {
        debug!(
            account = host.core().account_id,
            player = ?player_guid,
            spell_id,
            damage = damage_amount,
            "Skipping spellclick creature-caster damage because C++ EffectSchoolDMG requires alive target"
        );
        return Ok(());
    }
    host.sync_player_registry_state_like_cpp();
    if health_after != original_health {
        host.core()
            .send_player_health_update_like_cpp(player_guid, u64::from(health_after));
    }

    Ok(())
}

pub(crate) async fn apply_represented_spell_click_creature_damage_to_clickee_like_cpp<
    H: SpellClickHostLikeCpp,
>(
    host: &mut H,
    spell_id: i32,
    creature_guid: ObjectGuid,
    damage_amount: u32,
) -> Result<(), &'static str> {
    let account_id = host.core().account_id;
    let values_update = host.core_mut().mutate_world_creature(creature_guid, |creature| {
            if !creature.is_alive() {
                debug!(
                    account = account_id,
                    creature = ?creature_guid,
                    spell_id,
                    damage = damage_amount,
                    "Skipping spellclick creature-caster damage because C++ EffectSchoolDMG requires alive target"
                );
                return None;
            }
            let _died = creature.take_damage_before_death_state_like_cpp(damage_amount);
            Some(creature.unit().values_update())
        })
        .ok_or("Target creature not found")?;
    let Some(values_update) = values_update else {
        return Ok(());
    };

    if host
        .core()
        .client_visible_guids_like_cpp
        .contains(&creature_guid)
        && let Some(update) = host.represented_unit_values_update_to_update_object_like_cpp(
            creature_guid,
            host.core().player_map_id_like_cpp(),
            &values_update,
        )
    {
        host.send_packet(&update);
    }

    Ok(())
}
