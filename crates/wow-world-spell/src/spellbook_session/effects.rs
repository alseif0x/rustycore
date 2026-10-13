// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Spell-learning side effects of login, the start-all-spells option and the
//! talent rows, moved from the World `session/spell_state/effects.rs` (#1363
//! slice 9a).

use super::*;

pub const SPELL_AURA_INTERRUPT_FLAG2_CHANGE_TALENT_LIKE_CPP: u32 = 0x0000_4000;

pub fn load_represented_talent_row_with_spell_side_effects_like_cpp<H: SpellbookHostLikeCpp>(
    host: &mut H,
    talent_tabs: &TalentTabStore,
    talent_id: u32,
    rank: u8,
    talent_group: u8,
    known_spells: &mut Vec<i32>,
    dependent_spells: &mut HashSet<i32>,
) -> bool {
    let previous_rank = host
        .hub_ref()
        .player_talent_runtime_snapshot_like_cpp()
        .and_then(|runtime| {
            runtime
                .talent_group_like_cpp(talent_group)
                .and_then(|talents| talents.get(&talent_id).copied())
        });

    if !host.hub_mut().load_represented_talent_row_like_cpp(
        talent_tabs,
        talent_id,
        rank,
        talent_group,
    ) {
        return false;
    }

    if host.hub_ref().represented_active_talent_group_like_cpp() != Some(talent_group) {
        return true;
    }

    if let Some(previous_rank) = previous_rank {
        if let Some(previous_spell_id) =
            represented_talent_spell_id_like_cpp(host, talent_id, previous_rank)
        {
            known_spells.retain(|known_spell| *known_spell != previous_spell_id);
            dependent_spells.remove(&previous_spell_id);
            for trigger_spell in
                represented_direct_learn_spell_triggers_like_cpp(host, previous_spell_id)
            {
                known_spells.retain(|known_spell| *known_spell != trigger_spell);
                dependent_spells.remove(&trigger_spell);
            }
        }
    }

    if let Some(spell_id) = represented_talent_spell_id_like_cpp(host, talent_id, rank) {
        if !known_spells.contains(&spell_id) {
            known_spells.push(spell_id);
        }
        dependent_spells.insert(spell_id);
        for trigger_spell in represented_direct_learn_spell_triggers_like_cpp(host, spell_id) {
            if !known_spells.contains(&trigger_spell) {
                known_spells.push(trigger_spell);
            }
            dependent_spells.insert(trigger_spell);
        }
    }

    if let Some((overriden_spell_id, new_spell_id)) =
        represented_talent_override_spell_pair_like_cpp(host, talent_id)
    {
        add_represented_override_spell_like_cpp(host, overriden_spell_id, new_spell_id);
    }

    true
}
pub fn apply_represented_active_talent_spell_side_effects_like_cpp<H: SpellbookHostLikeCpp>(
    host: &mut H,
    talent_id: u32,
    previous_rank: Option<u8>,
    rank: u8,
    talent_group: u8,
) {
    if host.hub_ref().represented_active_talent_group_like_cpp() != Some(talent_group) {
        return;
    }

    if let Some(previous_rank) = previous_rank {
        if let Some(previous_spell_id) =
            represented_talent_spell_id_like_cpp(host, talent_id, previous_rank)
        {
            remove_known_spell_like_cpp(host, previous_spell_id);
            for trigger_spell in
                represented_direct_learn_spell_triggers_like_cpp(host, previous_spell_id)
            {
                remove_known_spell_like_cpp(host, trigger_spell);
            }
        }
    }

    if let Some(spell_id) = represented_talent_spell_id_like_cpp(host, talent_id, rank) {
        learn_known_spell_like_cpp(host, spell_id);
        for trigger_spell in represented_direct_learn_spell_triggers_like_cpp(host, spell_id) {
            learn_known_spell_like_cpp(host, trigger_spell);
        }
    }

    if let Some((overriden_spell_id, new_spell_id)) =
        represented_talent_override_spell_pair_like_cpp(host, talent_id)
    {
        add_represented_override_spell_like_cpp(host, overriden_spell_id, new_spell_id);
    }

    host.remove_auras_with_interrupt_flags_like_cpp(
        0,
        SPELL_AURA_INTERRUPT_FLAG2_CHANGE_TALENT_LIKE_CPP,
    );
}
