//! Item turn-in, destroy and charge actions, with the player class/team helpers they need.
//!
//! Split out of `character/mod.rs` under #584 (B5); items are unchanged.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExtendedCostItemTurninChange {
    Update {
        slot: u8,
        item_guid: ObjectGuid,
        db_guid: u64,
        new_count: u32,
    },
    Delete {
        slot: u8,
        item_guid: ObjectGuid,
        db_guid: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::handlers::character) enum DestroyItemCountAction {
    FullStack,
    PartialStack { new_count: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::handlers::character) struct DestroyQuestItemLikeCpp {
    pub(in crate::handlers::character) bag: u8,
    pub(in crate::handlers::character) slot: u8,
    pub(in crate::handlers::character) entry_id: u32,
    pub(in crate::handlers::character) count: u32,
}

pub(in crate::handlers::character) fn destroy_item_count_action(
    current_count: u32,
    requested_count: u32,
) -> DestroyItemCountAction {
    if requested_count != 0 && current_count > requested_count {
        return DestroyItemCountAction::PartialStack {
            new_count: current_count - requested_count,
        };
    }

    DestroyItemCountAction::FullStack
}

pub(in crate::handlers::character) fn item_spell_charges_db_string(charges: &[i32], effect_count: usize) -> String {
    let mut out = String::new();
    for charge in charges.iter().take(effect_count) {
        out.push_str(&charge.to_string());
        out.push(' ');
    }
    out
}

pub(in crate::handlers::character) fn player_class_mask(player_class: u8) -> u32 {
    player_class
        .checked_sub(1)
        .and_then(|shift| 1u32.checked_shl(u32::from(shift)))
        .unwrap_or(0)
}

pub(in crate::handlers::character) fn player_team_for_race_cpp(race: u8) -> Team {
    match race {
        // C++ resolves this from ChrRacesEntry::Alliance: 1 = Horde, 0 = Alliance.
        2 | 5 | 6 | 8 | 9 | 10 | 26 | 27 | 28 | 31 | 35 | 36 | 70 => Team::Horde,
        _ => Team::Alliance,
    }
}
