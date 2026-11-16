// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Raw aura-state values and deferred read projections.

use super::{AuraSubsystem, PER_CASTER_AURA_STATE_MASK};
use wow_core::ObjectGuid;

pub(crate) struct AuraStateValuesSnapshot {
    raw_mask: u32,
    per_state_caster_guids: Vec<(u8, Vec<ObjectGuid>)>,
}

impl AuraStateValuesSnapshot {
    pub(crate) fn capture(auras: &AuraSubsystem) -> Self {
        Self {
            raw_mask: auras.aura_state_mask,
            per_state_caster_guids: auras
                .aura_state_auras
                .iter()
                .map(|(&state, applied_auras)| {
                    (
                        state,
                        applied_auras.iter().map(|aura| aura.caster_guid).collect(),
                    )
                })
                .collect(),
        }
    }

    pub(crate) fn build_aura_state_update_for_target_like_cpp(&self, target: ObjectGuid) -> u32 {
        build_aura_state_update_for_target_like_cpp(
            self.raw_mask,
            self.per_state_caster_guids
                .iter()
                .map(|(state, casters)| (*state, casters.iter().copied())),
            target,
        )
    }

    pub(crate) fn aura_state_per_caster_contributions_like_cpp(&self) -> Vec<(ObjectGuid, u32)> {
        aura_state_per_caster_contributions_like_cpp(
            self.per_state_caster_guids
                .iter()
                .map(|(state, casters)| (*state, casters.iter().copied())),
        )
    }
}

pub(super) fn build_aura_state_update_for_target_like_cpp<Rows, Casters>(
    raw_mask: u32,
    rows: Rows,
    target: ObjectGuid,
) -> u32
where
    Rows: Iterator<Item = (u8, Casters)>,
    Casters: Iterator<Item = ObjectGuid>,
{
    let mut aura_states = raw_mask & !PER_CASTER_AURA_STATE_MASK;
    for (state, casters) in rows {
        let mask = 1 << (state - 1);
        if (mask & PER_CASTER_AURA_STATE_MASK) != 0
            && casters.into_iter().any(|caster| caster == target)
        {
            aura_states |= mask;
        }
    }
    aura_states
}

pub(super) fn aura_state_per_caster_contributions_like_cpp<Rows, Casters>(
    rows: Rows,
) -> Vec<(ObjectGuid, u32)>
where
    Rows: Iterator<Item = (u8, Casters)>,
    Casters: Iterator<Item = ObjectGuid>,
{
    let mut contributions = Vec::new();
    for (state, casters) in rows {
        let mask = 1_u32 << (state - 1);
        if mask & PER_CASTER_AURA_STATE_MASK != 0 {
            contributions.extend(casters.into_iter().map(|caster| (caster, mask)));
        }
    }
    contributions.sort_unstable();
    contributions.dedup();
    contributions
}
