// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Pure scalar projection for selected aura wire updates.

use wow_core::ObjectGuid;
use wow_entities::{
    AuraApplicationLikeCpp as AuraApplication, AuraCastProvenanceLikeCpp as AuraCastProvenance,
    AuraThreatSnapshotLikeCpp as AuraThreatSnapshot,
};
use wow_packet::packets::misc::{AuraDataInfoLikeCpp, AuraInfoLikeCpp};

const AFLAG_NOCASTER: u16 = 0x0001;
const AFLAG_DURATION: u16 = 0x0004;
const AFLAG_SCALABLE: u16 = 0x0008;
const ATTR5_DO_NOT_DISPLAY_DURATION: u32 = 0x0000_0400;
const ATTR11_SCALES_WITH_ITEM_LEVEL: u32 = 0x0000_0004;
const SELECTED_EFFECT_MASK: u32 = 0b0111;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SelectedAuraWireFacts {
    pub(super) captured_caster_level: u8,
    pub(super) cast_item_level: i32,
    pub(super) catalog_stack_amount: u32,
    pub(super) canonical_charges: u8,
    pub(super) attributes5: u32,
    pub(super) attributes11: u32,
}

pub(super) fn selected_aura_info(
    aura: &AuraApplication,
    provenance: &AuraCastProvenance,
    snapshot: &AuraThreatSnapshot,
    facts: SelectedAuraWireFacts,
) -> Result<AuraInfoLikeCpp, &'static str> {
    if provenance.cast_id.is_empty() {
        return Err("Selected Aura cast identity is unavailable");
    }
    if aura.effect_mask == 0 || aura.effect_mask & !SELECTED_EFFECT_MASK != 0 {
        return Err("Selected Aura application has an invalid active effect mask");
    }
    if aura.aura_flags > u32::from(u16::MAX) {
        return Err("Selected Aura flags do not fit the AuraUpdate wire field");
    }

    let mut effect_amounts = [0_i32; 3];
    let mut snapshot_mask = 0_u32;
    for &(bit, _, amount, _) in snapshot.effects() {
        if bit.count_ones() != 1 || bit & !aura.effect_mask != 0 || snapshot_mask & bit != 0 {
            return Err("Selected Aura effect snapshot has invalid active-effect coverage");
        }
        let index = bit.trailing_zeros() as usize;
        if index >= effect_amounts.len() {
            return Err("Selected Aura effect snapshot has invalid active-effect coverage");
        }
        effect_amounts[index] = amount;
        snapshot_mask |= bit;
    }
    if snapshot_mask != aura.effect_mask {
        return Err("Selected Aura effect snapshot does not cover its active effects");
    }

    let mut flags = aura.aura_flags as u16 & !AFLAG_DURATION;
    let has_duration =
        aura.duration_total > 0 && facts.attributes5 & ATTR5_DO_NOT_DISPLAY_DURATION == 0;
    if has_duration {
        flags |= AFLAG_DURATION;
    }

    if !aura.caster_guid.is_unit() && flags & AFLAG_NOCASTER != 0 {
        return Err("Non-unit selected Aura caster cannot use NOCASTER flags");
    }
    let caster_guid = if aura.caster_guid.is_unit() {
        aura.caster_guid
    } else {
        ObjectGuid::EMPTY
    };

    let cast_level = if facts.attributes11 & ATTR11_SCALES_WITH_ITEM_LEVEL != 0 {
        facts.cast_item_level as u16
    } else {
        u16::from(facts.captured_caster_level)
    };
    let applications = if facts.catalog_stack_amount > 0 || aura.stack_count > 1 {
        aura.stack_count
    } else {
        facts.canonical_charges
    };
    let (duration_ms, remaining_ms) = if has_duration {
        (Some(aura.duration_total), Some(aura.duration_remaining))
    } else {
        (None, None)
    };
    let points = if flags & AFLAG_SCALABLE != 0 {
        let point_count = (u32::BITS - aura.effect_mask.leading_zeros()) as usize;
        effect_amounts[..point_count]
            .iter()
            .map(|amount| *amount as f32)
            .collect()
    } else {
        Vec::new()
    };

    Ok(AuraInfoLikeCpp {
        slot: aura.slot,
        aura_data: Some(AuraDataInfoLikeCpp {
            cast_id: provenance.cast_id,
            spell_id: aura.spell_id,
            spell_visual_id: provenance.spell_visual_id,
            flags,
            active_flags: aura.effect_mask,
            caster_guid,
            cast_level,
            applications,
            duration_ms,
            remaining_ms,
            points,
        }),
    })
}

#[cfg(test)]
#[path = "../../../unit_tests/session/spell_state/selected_wire.rs"]
mod tests;
