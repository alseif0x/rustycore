//! 02245dcd CharacterHandler.cpp:559-665, RaceMask.h:100-154.
//! Creation has no Player/rewarded quests. Item appearance ownership must be
//! supplied from the admitted account, never inferred from a DB2 requirement.
use std::collections::BTreeSet;
use wow_data::forever_appearance::{AppearanceCatalog, Requirement};
use wow_packet::forever::character_create::CharacterCreatePayload;

pub fn validate_creation_appearance(
    catalog: &AppearanceCatalog,
    create: &CharacterCreatePayload,
    owned_item_appearances: &BTreeSet<u32>,
) -> bool {
    let Some(options) = catalog.options(create.race, create.sex) else {
        return false;
    };
    if !(1..=32).contains(&create.class) {
        return false;
    }
    let mut previous = 0;
    for selected in &create.customizations {
        if selected.option_id <= previous {
            return false;
        }
        previous = selected.option_id;
        let Some(option) = options
            .iter()
            .find(|option| option.id == selected.option_id)
        else {
            return false;
        };
        if let Some(req) = catalog.requirement(option.requirement) {
            if !meets(catalog, req, create, false, owned_item_appearances) {
                return false;
            }
        }
        let Some(choices) = catalog.choices(selected.option_id) else {
            return false;
        };
        let Some(choice) = choices
            .iter()
            .find(|choice| choice.id == selected.choice_id)
        else {
            return false;
        };
        if let Some(req) = catalog.requirement(choice.requirement) {
            if !meets(catalog, req, create, true, owned_item_appearances) {
                return false;
            }
        }
    }
    true
}

fn meets(
    catalog: &AppearanceCatalog,
    req: &Requirement,
    create: &CharacterCreatePayload,
    check_dependencies: bool,
    owned: &BTreeSet<u32>,
) -> bool {
    if req.flags & 1 == 0 {
        return true;
    }
    if req.class_mask != 0 && (req.class_mask as u32 & (1 << (create.class - 1))) == 0 {
        return false;
    }
    if create.race != 0 && req.race_mask != [0; 2] && req.race_mask != [u32::MAX; 2] {
        let Some(bit) = race_bit(create.race) else {
            return false;
        };
        if req.race_mask[bit / 32] & (1 << (bit % 32)) == 0 {
            return false;
        }
    }
    if req.achievement != 0 || req.quest != 0 {
        return false;
    }
    if req.item_appearance != 0 && !owned.contains(&(req.item_appearance as u32)) {
        return false;
    }
    if check_dependencies {
        if let Some(groups) = catalog.required_choices(req.id) {
            for choices in groups.values() {
                if !choices.iter().any(|id| {
                    create
                        .customizations
                        .iter()
                        .any(|selected| selected.choice_id == *id)
                }) {
                    return false;
                }
            }
        }
    }
    true
}

fn race_bit(race: u8) -> Option<usize> {
    Some(match race {
        1..=11 | 22 | 24..=32 => usize::from(race - 1),
        34 => 11,
        35 => 12,
        36 => 13,
        37 => 14,
        70 => 15,
        52 => 16,
        84 => 17,
        85 => 18,
        91 => 19,
        86 => 20,
        95 => 32,
        96 => 33,
        _ => return None,
    })
}

#[cfg(test)]
mod tests;
