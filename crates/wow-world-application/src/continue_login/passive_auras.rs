// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The login passive-aura replays (C++ `Player::_LoadSpells` /
//! `Player::LearnSpell` passive casts at login and the previous-rank passive
//! auras; #1363 slice 12d, moved from the World
//! `session/spell_state/aura_application.rs`). The aura application itself is
//! the host's `apply_aura_with_effect_mask_like_cpp`.

use wow_world_core::session::AFLAG_NOCASTER_LIKE_CPP;
use wow_world_loot_ops::LootSessionHostLikeCpp;
use wow_world_spell::unit_owned_apply_aura_effect_mask_like_cpp;

use super::ContinueLoginHostLikeCpp;

pub fn apply_login_passive_known_spell_auras_like_cpp<H: ContinueLoginHostLikeCpp>(
    host: &mut H,
) -> usize {
    let Some(player_guid) = host.player_guid() else {
        return 0;
    };
    let Some(spell_store) = host.hub_ref().catalogs.spell_store().cloned() else {
        return 0;
    };

    let mut applied = 0usize;
    for spell_id in wow_world_spell::known_spells_like_cpp(host).to_vec() {
        if spell_id <= 0
            || !spell_store.is_passive_like_cpp(spell_id)
            || host
                .hub_ref()
                .player_has_visible_aura_spell_like_cpp(spell_id)
                != Some(false)
        {
            continue;
        }

        let Some(spell_info) = spell_store.get(spell_id).cloned() else {
            continue;
        };
        let effect_mask = unit_owned_apply_aura_effect_mask_like_cpp(&spell_info);
        if effect_mask == 0 {
            continue;
        }

        if let Some(equipped) = host
            .hub_ref()
            .catalogs
            .spell_catalogs
            .spell_equipped_items_store
            .as_ref()
            .and_then(|store| store.entry_for_spell_id_like_cpp(spell_id))
            .filter(|entry| entry.equipped_item_class >= 0)
            .cloned()
        {
            let (inventory, hub) = LootSessionHostLikeCpp::split_inventory_ref(host);
            if !inventory.represented_has_item_fit_to_spell_requirements_like_cpp(hub, &equipped) {
                continue;
            }
        } else if !wow_world_spell::represented_login_passive_spell_cast_gate_like_cpp(
            host, spell_id,
        ) {
            continue;
        }

        if host
            .apply_aura_with_effect_mask_like_cpp(
                spell_id,
                player_guid,
                0,
                AFLAG_NOCASTER_LIKE_CPP | 0x0000_0100 | 0x0000_0200,
                effect_mask,
            )
            .is_ok()
        {
            applied += 1;
        }
    }

    applied
}

pub fn apply_loaded_known_spell_previous_rank_passive_auras_like_cpp<
    H: ContinueLoginHostLikeCpp,
>(
    host: &mut H,
    known_spells: &[i32],
) -> usize {
    let Some(player_guid) = host.player_guid() else {
        return 0;
    };
    let Some(spell_store) = host.hub_ref().catalogs.spell_store().cloned() else {
        return 0;
    };

    let mut applied = 0usize;
    for &spell_id in known_spells {
        let Ok(mut previous_spell_id) = u32::try_from(spell_id).map(|spell_id| {
            host.hub_ref()
                .catalogs
                .prev_spell_in_chain_like_cpp(spell_id)
        }) else {
            continue;
        };

        while previous_spell_id != 0 {
            let Ok(previous_spell_i32) = i32::try_from(previous_spell_id) else {
                break;
            };
            if spell_store.is_passive_like_cpp(previous_spell_i32)
                && host
                    .hub_ref()
                    .player_has_visible_aura_spell_like_cpp(previous_spell_i32)
                    == Some(false)
                && wow_world_spell::represented_login_passive_spell_cast_gate_like_cpp(
                    host,
                    previous_spell_i32,
                )
                && let Some(spell_info) = spell_store.get(previous_spell_i32).cloned()
            {
                let effect_mask = unit_owned_apply_aura_effect_mask_like_cpp(&spell_info);
                if effect_mask != 0
                    && host
                        .apply_aura_with_effect_mask_like_cpp(
                            previous_spell_i32,
                            player_guid,
                            0,
                            AFLAG_NOCASTER_LIKE_CPP | 0x0000_0100 | 0x0000_0200,
                            effect_mask,
                        )
                        .is_ok()
                {
                    applied += 1;
                }
            }

            previous_spell_id = host
                .hub_ref()
                .catalogs
                .prev_spell_in_chain_like_cpp(previous_spell_id);
        }
    }

    applied
}
