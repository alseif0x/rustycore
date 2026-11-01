// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Represented talent learn and reset, moved from the World
//! `session/progression/talents.rs`, with the active-talent spell removal
//! from `session/movement/state.rs` (#1363 slice 9b).

use wow_data::TalentTabStore;
use wow_world_core::session::MAX_SPECIALIZATIONS_LIKE_CPP;

use super::TalentHostLikeCpp;
use crate::{
    apply_represented_active_talent_spell_side_effects_like_cpp,
    clear_represented_trait_and_override_state_like_cpp,
    invalidate_represented_spell_acquisition_auxiliary_authority_like_cpp,
    remove_known_spell_like_cpp, remove_represented_override_spell_like_cpp,
    represented_direct_learn_spell_triggers_like_cpp,
    represented_talent_override_spell_pair_like_cpp, represented_talent_spell_id_like_cpp,
};

const LANG_RESET_TALENTS_LIKE_CPP: u32 = 216;
const LANG_RESET_TALENTS_TEXT_LIKE_CPP: &str = "Your talents have been reset.";

pub fn reset_represented_talents_like_cpp<H: TalentHostLikeCpp>(host: &mut H) {
    let _ = host
        .hub_mut()
        .mutate_player_talent_runtime_like_cpp(|runtime| {
            runtime.clear_talents_like_cpp();
        });
    // Login reconstructs a fresh C++ Player after this reset. Retaining the
    // previous character's runtime edges would affect GetCastSpellInfo, and
    // retaining per-spell traits could contaminate a coincident spell ID.
    let _ = clear_represented_trait_and_override_state_like_cpp(host);
    invalidate_represented_spell_acquisition_auxiliary_authority_like_cpp(host);
}
pub(crate) fn reset_represented_active_talents_like_cpp<H: TalentHostLikeCpp>(
    host: &mut H,
) -> bool {
    let Some(talent_group) = host.hub_ref().represented_active_talent_group_like_cpp() else {
        return false;
    };
    let talent_group_index = usize::from(talent_group);
    if talent_group_index >= MAX_SPECIALIZATIONS_LIKE_CPP {
        return false;
    }

    let Some(active_talents) = host
        .hub_mut()
        .mutate_player_talent_runtime_like_cpp(|runtime| {
            runtime.take_talent_group_like_cpp(talent_group)
        })
        .flatten()
    else {
        return false;
    };
    for (talent_id, rank) in active_talents {
        remove_represented_active_talent_side_effects_like_cpp(host, talent_id, rank);
    }
    host.refresh_represented_talent_points_like_cpp();
    true
}
pub fn apply_represented_login_talent_reset_if_needed_like_cpp<H: TalentHostLikeCpp>(
    host: &mut H,
) -> bool {
    const AT_LOGIN_RESET_TALENTS_LIKE_CPP: u16 = 0x004;

    if !host
        .resolved_represented_at_login_flags_like_cpp()
        .is_some_and(|flags| (flags & AT_LOGIN_RESET_TALENTS_LIKE_CPP) != 0)
    {
        return false;
    }

    host.hub_mut()
        .record_represented_talent_reset_script_hook_like_cpp(true);
    host.remove_represented_at_login_flag_like_cpp(AT_LOGIN_RESET_TALENTS_LIKE_CPP, true);
    host.hub_mut().remove_represented_pet_not_in_slot_like_cpp();

    if reset_represented_active_talents_like_cpp(host) {
        let Some(talent_data) = host.hub_ref().resolved_update_talent_data_packet_like_cpp() else {
            return false;
        };
        host.send_packet(&talent_data);
        host.core()
            .send_notification_like_cpp(reset_talents_notification_text_like_cpp(host));
        return true;
    }

    false
}
pub fn learn_represented_talent_like_cpp<H: TalentHostLikeCpp>(
    host: &mut H,
    talent_tabs: &TalentTabStore,
    talent_id: u32,
    requested_rank: u16,
) -> bool {
    if !host.represented_talents_loaded_like_cpp() {
        return false;
    }

    let Ok(rank) = u8::try_from(requested_rank) else {
        return false;
    };

    if !host
        .hub_ref()
        .validate_represented_talent_learn_like_cpp(talent_id, rank)
    {
        return false;
    }

    let Some(runtime) = host.hub_ref().player_talent_runtime_snapshot_like_cpp() else {
        return false;
    };
    let talent_group = runtime.active_group_like_cpp();
    let previous_rank = runtime
        .talent_group_like_cpp(talent_group)
        .and_then(|talents| talents.get(&talent_id).copied());
    let learned = host.hub_mut().load_represented_talent_row_like_cpp(
        talent_tabs,
        talent_id,
        rank,
        talent_group,
    );
    if learned {
        apply_represented_active_talent_spell_side_effects_like_cpp(
            host,
            talent_id,
            previous_rank,
            rank,
            talent_group,
        );
        host.refresh_represented_talent_points_like_cpp();
    }
    learned
}
fn reset_talents_notification_text_like_cpp<H: TalentHostLikeCpp>(host: &H) -> String {
    let text = host.trinity_string_like_cpp(LANG_RESET_TALENTS_LIKE_CPP);
    if text == "<error>" {
        LANG_RESET_TALENTS_TEXT_LIKE_CPP.to_string()
    } else {
        text.to_string()
    }
}

pub fn remove_represented_active_talent_side_effects_like_cpp<H: TalentHostLikeCpp>(
    host: &mut H,
    talent_id: u32,
    rank: u8,
) {
    if let Some(spell_id) = represented_talent_spell_id_like_cpp(host, talent_id, rank) {
        remove_known_spell_like_cpp(host, spell_id);
        for trigger_spell in represented_direct_learn_spell_triggers_like_cpp(host, spell_id) {
            remove_known_spell_like_cpp(host, trigger_spell);
        }
    }

    if let Some((overriden_spell_id, new_spell_id)) =
        represented_talent_override_spell_pair_like_cpp(host, talent_id)
    {
        remove_represented_override_spell_like_cpp(host, overriden_spell_id, new_spell_id);
    }
}
