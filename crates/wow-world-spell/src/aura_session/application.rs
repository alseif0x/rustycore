// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Applying, removing and expiring represented auras, moved from the World
//! `session/spell_state/aura_application.rs` (#1363 slice 9b).

use std::time::Instant;

use tracing::debug;
use wow_core::ObjectGuid;
use wow_entities::{AuraApplicationLikeCpp as AuraApplication, RepresentedAuraEffectLikeCpp};
use wow_world_entities::SPELL_AURA_INTERRUPT_FLAG_TURNING_LIKE_CPP;

use super::{
    AuraHostLikeCpp, insert_player_visible_aura_like_cpp, next_player_visible_aura_slot_like_cpp,
    send_aura_update_applied,
};
use crate::represented_aura_effect_amounts_like_cpp;

pub const SPELL_AURA_INTERRUPT_FLAG_MOVING_LIKE_CPP: u32 = 0x0000_0008;
pub(crate) const SPELL_AURA_INTERRUPT_FLAG_MOVING_OR_TURNING_LIKE_CPP: u32 =
    SPELL_AURA_INTERRUPT_FLAG_MOVING_LIKE_CPP | SPELL_AURA_INTERRUPT_FLAG_TURNING_LIKE_CPP;

pub fn remove_indoor_outdoor_auras_for_current_position_represented_like_cpp<H: AuraHostLikeCpp>(
    host: &mut H,
) -> usize {
    if !host.hub_ref().config.vmap_indoor_check_like_cpp {
        return 0;
    }

    let Some(is_outdoors) = host
        .hub_ref()
        .player_world_local_state_like_cpp()
        .and_then(|state| state.is_outdoors_like_cpp())
    else {
        return 0;
    };

    let attribute = if is_outdoors {
        wow_data::spell::attributes::SPELL_ATTR0_ONLY_INDOORS
    } else {
        wow_data::spell::attributes::SPELL_ATTR0_ONLY_OUTDOORS
    };
    remove_represented_auras_with_attribute0_like_cpp(host, attribute)
}

pub fn remove_represented_auras_due_to_spell_like_cpp<H: AuraHostLikeCpp>(
    host: &mut H,
    spell_id: i32,
) -> usize {
    let Some(visible_auras) = host.hub_ref().resolved_player_visible_auras_like_cpp() else {
        return 0;
    };
    let slots = visible_auras
        .values()
        .filter_map(|aura| (aura.spell_id == spell_id).then_some(aura.slot))
        .collect::<Vec<_>>();
    let removed = slots.len();
    for slot in slots {
        let _ = host.remove_aura(slot);
    }
    removed
}

pub fn apply_represented_provide_spell_focus_aura_like_cpp<H: AuraHostLikeCpp>(
    host: &mut H,
    spell_id: i32,
    caster_guid: ObjectGuid,
    effect: &wow_data::SpellEffectInfo,
) -> Result<(), &'static str> {
    let slot = next_player_visible_aura_slot_like_cpp(host)
        .ok_or("No free aura slots or missing Player aura owner")?;

    let aura = AuraApplication {
        spell_id,
        difficulty_id: host.core().current_map_difficulty_id_like_cpp(),
        caster_guid,
        slot,
        duration_total: 30_000,
        duration_remaining: 30_000,
        stack_count: 1,
        aura_flags: 0x0000_0001,
        effect_mask: 1u32 << effect.effect_index,
        aura_interrupt_flags: 0,
        aura_interrupt_flags2: 0,
        represented_effect: Some(RepresentedAuraEffectLikeCpp::ProvideSpellFocus),
        represented_amount: effect.effect_base_points,
        represented_effect_amounts: represented_aura_effect_amounts_like_cpp(effect),
        represented_misc_value: Some(effect.effect_misc_value_1),
        represented_multiplier: 1.0,
        applied_at: Instant::now(),
    };

    if !insert_player_visible_aura_like_cpp(host, aura) {
        return Err("Missing Player aura owner");
    }
    send_aura_update_applied(host, spell_id, slot, caster_guid, 30_000, 0x0000_0001, 0x1);

    Ok(())
}

pub(crate) fn remove_represented_auras_with_attribute0_like_cpp<H: AuraHostLikeCpp>(
    host: &mut H,
    attribute: u32,
) -> usize {
    let Some(spell_store) = host.hub_ref().catalogs.spell_catalogs.spell_store.as_ref() else {
        return 0;
    };
    let Some(visible_auras) = host.hub_ref().resolved_player_visible_auras_like_cpp() else {
        return 0;
    };

    let slots: Vec<u8> = visible_auras
        .values()
        .filter_map(|aura| {
            spell_store
                .has_attribute0_like_cpp(aura.spell_id, attribute)
                .then_some(aura.slot)
        })
        .collect();

    let removed = slots.len();
    for slot in slots {
        let _ = host.remove_aura(slot);
    }
    removed
}

pub fn remove_moving_or_turning_interrupt_auras_for_far_teleport_like_cpp<H: AuraHostLikeCpp>(
    host: &mut H,
) -> usize {
    let represented_removed = remove_auras_with_interrupt_flags_like_cpp(
        host,
        SPELL_AURA_INTERRUPT_FLAG_MOVING_OR_TURNING_LIKE_CPP,
        0,
    );
    let canonical_removed = host
        .core()
        .mutate_canonical_player_like_cpp(|player| {
            player
                .unit_mut()
                .subsystems_mut()
                .auras
                .remove_interruptible_auras(SPELL_AURA_INTERRUPT_FLAG_MOVING_OR_TURNING_LIKE_CPP, 0)
                .len()
        })
        .unwrap_or(0);

    represented_removed + canonical_removed
}

pub fn remove_auras_with_interrupt_flags_like_cpp<H: AuraHostLikeCpp>(
    host: &mut H,
    flags: u32,
    flags2: u32,
) -> usize {
    let Some(visible_auras) = host.hub_ref().resolved_player_visible_auras_like_cpp() else {
        return 0;
    };
    let slots: Vec<u8> = visible_auras
        .values()
        .filter(|aura| {
            (flags != 0 && aura.aura_interrupt_flags & flags != 0)
                || (flags2 != 0 && aura.aura_interrupt_flags2 & flags2 != 0)
        })
        .map(|aura| aura.slot)
        .collect();

    let removed = slots.len();
    for slot in slots {
        let _ = host.remove_aura(slot);
    }
    removed
}

/// Check all active auras for expiry and remove those whose duration has elapsed.
/// Called from the synchronous tick loop (~every 200ms via creature_tick).
pub fn tick_auras<H: AuraHostLikeCpp>(host: &mut H) {
    // C++ `Creature::Update` owns creature aura lifetimes; this session
    // expires the creature auras it applied through the same wall-clock
    // sweep it uses for its own auras.
    host.tick_represented_creature_auras_like_cpp();
    let Some(visible_auras) = host.hub_ref().resolved_player_visible_auras_like_cpp() else {
        return;
    };
    if visible_auras.is_empty() {
        return;
    }

    // Collect expired slots (avoid borrow conflict)
    let expired: Vec<u8> = visible_auras
        .values()
        .filter(|a| {
            // Permanent auras (duration_total == 0) never expire. SetDuration
            // changes the remaining lifetime while retaining the maximum.
            a.duration_total > 0
                && a.applied_at.elapsed().as_millis() >= u128::from(a.duration_remaining)
        })
        .map(|a| a.slot)
        .collect();

    for slot in expired {
        let spell_id = visible_auras.get(&slot).map(|a| a.spell_id).unwrap_or(0);
        let _ = host.remove_aura(slot);
        debug!(
            account = host.core().account_id,
            slot = slot,
            spell_id = spell_id,
            "Aura expired"
        );
    }

    // `Unit::m_visibleAurasToUpdate` is a slot-ordered set
    // (`Unit.h:638-639`, `Unit.cpp:13566-13569`). The Rust subsystem already
    // owns the pending slots; consume only the two selected resurrection
    // spells here and leave every other pending aura for its existing owner.
    let pending = host
        .core()
        .with_owned_player_mut_like_cpp(|player| {
            let auras = &mut player.unit_mut().subsystems_mut().auras;
            let mut slots = auras
                .visible_auras_to_update
                .iter()
                .copied()
                .filter(|slot| {
                    auras
                        .runtime_application_like_cpp(*slot)
                        .is_some_and(|aura| matches!(aura.spell_id, 8326 | 20584))
                })
                .collect::<Vec<_>>();
            slots.sort_unstable();
            let applications = slots
                .iter()
                .filter_map(|slot| {
                    Some((
                        auras.runtime_application_like_cpp(*slot)?.clone(),
                        auras.aura_cast_provenance_like_cpp(*slot),
                    ))
                })
                .collect::<Vec<_>>();
            for slot in slots {
                auras.visible_auras_to_update.remove(&slot);
            }
            applications
        })
        .unwrap_or_default();
    for (aura, provenance) in pending {
        let (state, hub) = host.split_spell_state_ref();
        state.send_aura_application_update_with_provenance(hub, &aura, provenance);
    }
}
