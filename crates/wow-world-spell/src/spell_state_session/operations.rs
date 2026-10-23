// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The self-resurrection spell check, the far-teleport cast interrupt
//! (C++ `Unit::InterruptNonMeleeSpells` on `Player::TeleportTo`) and the login
//! proficiency replay (#1363 slice 12b3), moved from the World
//! `session/spell_state` spell and spellbook files.

use wow_entities::PlayerResurrectionStateLikeCpp;

use crate::{
    SpellExecutionHostLikeCpp, SpellStateHostLikeCpp, apply_proficiency_effect_like_cpp,
    player_persistent_capability_state_snapshot_like_cpp,
};

// The resurrection-state snapshot (#1363 slice 12d; moved from the World
// `session/combat/death.rs`).
pub fn player_resurrection_state_snapshot_like_cpp<H: SpellExecutionHostLikeCpp + ?Sized>(
    host: &H,
) -> Option<PlayerResurrectionStateLikeCpp> {
    let canonical = host
        .core()
        .with_owned_player_like_cpp(|player| player.resurrection_state_like_cpp().clone());
    #[cfg(any(test, feature = "test-fixtures"))]
    if host.test_fixtures_ref_like_cpp().is_some()
        && canonical.is_none()
        && host.core().player_handle_like_cpp.is_none()
    {
        return Some(PlayerResurrectionStateLikeCpp {
            request: host
                .hub_ref()
                .fixtures
                .combat
                .represented_resurrection_request_like_cpp,
            delayed_after_teleport: host
                .hub_ref()
                .fixtures
                .combat
                .represented_delayed_resurrection_after_teleport_like_cpp,
            self_res_spells: host
                .spell_state()
                .represented_self_res_spells_for_test_like_cpp()
                .clone(),
            death_timer_active: host
                .hub_ref()
                .fixtures
                .combat
                .represented_death_timer_active_like_cpp,
            area_spirit_healer_guid: host
                .hub_ref()
                .fixtures
                .combat
                .area_spirit_healer_guid_like_cpp,
            ..PlayerResurrectionStateLikeCpp::default()
        });
    }
    canonical
}

pub fn has_represented_self_res_spell_like_cpp<H: SpellExecutionHostLikeCpp>(
    host: &H,
    spell_id: i32,
) -> bool {
    player_resurrection_state_snapshot_like_cpp(host)
        .is_some_and(|state| state.self_res_spells.contains(&spell_id))
}

pub fn interrupt_non_melee_spells_for_far_teleport_like_cpp<H: SpellStateHostLikeCpp>(
    host: &mut H,
) -> bool {
    let session_cast_interrupted = host.interrupt_player_cast_like_cpp(None);
    let canonical_spells_interrupted = host
        .core()
        .mutate_canonical_player_like_cpp(|player| {
            let unit = player.unit_mut();
            if !unit.is_non_melee_spell_cast_like_cpp(true, false, false, true) {
                return false;
            }
            !unit.interrupt_non_melee_spells(None, true, true).is_empty()
        })
        .unwrap_or(false);

    session_cast_interrupted || canonical_spells_interrupted
}

pub fn apply_login_known_spell_proficiencies_like_cpp<H: SpellStateHostLikeCpp>(
    host: &mut H,
    known_spells: &[i32],
) -> usize {
    if host.player_guid().is_none()
        || player_persistent_capability_state_snapshot_like_cpp(host).is_none()
    {
        return 0;
    }
    let Some(spell_store) = host.hub_ref().catalogs.spell_store().cloned() else {
        return 0;
    };

    let mut applied = 0usize;
    for &spell_id in known_spells {
        let Some(spell_info) = spell_store.get(spell_id) else {
            continue;
        };
        if spell_info.effect_type != wow_data::spell::spell_effect_types::SPELL_EFFECT_PROFICIENCY
            && !spell_info
                .has_effect_like_cpp(wow_data::spell::spell_effect_types::SPELL_EFFECT_PROFICIENCY)
        {
            continue;
        }

        let Some(before) = player_persistent_capability_state_snapshot_like_cpp(host) else {
            return applied;
        };
        if apply_proficiency_effect_like_cpp(host, spell_id).is_ok()
            && player_persistent_capability_state_snapshot_like_cpp(host).is_some_and(|after| {
                after.weapon_proficiency != before.weapon_proficiency
                    || after.armor_proficiency != before.armor_proficiency
            })
        {
            applied += 1;
        }
    }

    applied
}
