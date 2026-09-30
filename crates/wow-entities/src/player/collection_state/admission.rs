//! The complete represented appearance admission operation over lazy borrowed sources.
//!
//! a5f8da2e CollectionMgr.cpp:649-730 and Player.cpp:11069-11125.
//! The represented holiday/artifact-specialization and ItemSpecStats gaps remain.

use super::*;
use wow_constants::{
    InventoryType, ItemClass, ItemFlags2, ItemFlags3, ItemQuality, ItemSubClassArmor,
    ItemSubClassWeapon,
};

#[derive(Debug, Clone, Copy)]
pub struct AppearanceModifiedFacts {
    pub item_id: i32,
    pub transmog_source_type_enum: i32,
}

#[derive(Debug, Clone, Copy)]
pub struct AppearanceSearchFacts {
    pub allowable_race: i64,
    pub required_level: i8,
    pub required_skill: u16,
    pub required_skill_rank: u16,
    pub required_ability: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct AppearanceSparseFacts {
    pub flags: [u32; 4],
    pub required_reputation_faction: u16,
    pub required_reputation_rank: i32,
    pub allowable_class: i16,
}

#[derive(Debug, Clone, Copy)]
pub struct AppearanceStorageFacts {
    pub class_id: ItemClass,
    pub inventory_type: InventoryType,
}

/// One appearance-admission read port. Each query executes at the original gate;
/// implementations project only the fields that gate consumes, without a catalog snapshot.
pub trait AppearanceAdmissionSource {
    fn modified_appearance(&self, id: u32) -> Option<AppearanceModifiedFacts>;
    fn search_name(&self, item_id: u32) -> Option<AppearanceSearchFacts>;
    fn item_subclass(&self, item_id: u32) -> Option<u8>;
    fn sparse_template(&self, item_id: u32) -> Option<AppearanceSparseFacts>;
    fn storage_template(&self, item_id: u32) -> Option<AppearanceStorageFacts>;
    fn has_player_guid(&self) -> bool;
    fn player_race(&self) -> u8;
    fn team_for_race(&self, race: u8) -> u32;
    fn player_level(&self) -> u8;
    fn player_skill(&self, skill: u16) -> Option<u16>;
    fn knows_spell(&self, spell_id: i32) -> bool;
    fn reputation_rank(&self, faction_id: u32) -> Option<u32>;
    fn learning_effects(&self, item_id: u32) -> Vec<(u8, i32)>;
    fn player_class(&self) -> u8;
    fn class_mask(&self, class_id: u8) -> u32;
    fn item_quality(&self, item_id: u32) -> Option<i8>;
    fn weapon_proficiency(&self) -> Option<u32>;
    fn armor_class_mask(&self, subclass: u32) -> u32;
    fn permanent_appearance_exists(&self, id: u32) -> Option<bool>;
}

impl PlayerCollectionStateLikeCpp {
    /// Complete represented CanAddAppearance, retaining every lazy gate and fallback.
    pub fn can_add_appearance(
        source: &impl AppearanceAdmissionSource,
        item_modified_appearance_id: u32,
    ) -> bool {
        let Some(item_modified_appearance) =
            source.modified_appearance(item_modified_appearance_id)
        else {
            return false;
        };

        if matches!(item_modified_appearance.transmog_source_type_enum, 6 | 9) {
            return false;
        }

        let Ok(item_id) = u32::try_from(item_modified_appearance.item_id) else {
            return false;
        };
        let Some(search_template) = source.search_name(item_id) else {
            return false;
        };

        let Some(item_record) = source.item_subclass(item_id) else {
            return false;
        };
        let Some(sparse_template) = source.sparse_template(item_id) else {
            return false;
        };
        let Some(template) = source.storage_template(item_id) else {
            return false;
        };
        if !source.has_player_guid() {
            return false;
        }

        // C++ `CollectionMgr::CanAddAppearance` first runs
        // `Player::CanUseItem(ItemTemplate const*)` (`Player.cpp:11069-11125`).
        // The represented template gates are the internal/faction flags, the
        // allowable class/race masks and the required skill, spell and level
        // checks; the holiday, reputation, learning-effect and artifact
        // specialization gates remain separate slices.
        let use_flags2 = sparse_template.flags[1];
        if (use_flags2 & ItemFlags2::InternalItem as u32) != 0 {
            return false;
        }
        let player_team = source.team_for_race(source.player_race());
        if (use_flags2 & ItemFlags2::FactionHorde as u32) != 0
            && player_team != crate::TEAM_HORDE_ID
        {
            return false;
        }
        if (use_flags2 & ItemFlags2::FactionAlliance as u32) != 0
            && player_team != crate::TEAM_ALLIANCE_ID
        {
            return false;
        }
        let player_race_mask = source
            .player_race()
            .checked_sub(1)
            .and_then(|shift| 1i64.checked_shl(u32::from(shift)))
            .unwrap_or(0);
        if search_template.allowable_race != 0
            && (search_template.allowable_race & player_race_mask) == 0
        {
            return false;
        }
        if search_template.required_level > 0
            && source.player_level()
                < u8::try_from(search_template.required_level).unwrap_or(u8::MAX)
        {
            return false;
        }
        if search_template.required_skill != 0 {
            let Some(skill_value) = u16::try_from(search_template.required_skill)
                .ok()
                .and_then(|skill| source.player_skill(skill))
            else {
                return false;
            };
            if u32::from(skill_value) < u32::from(search_template.required_skill_rank) {
                return false;
            }
        }
        if search_template.required_ability != 0
            && !i32::try_from(search_template.required_ability)
                .ok()
                .is_some_and(|spell_id| source.knows_spell(spell_id))
        {
            return false;
        }
        if sparse_template.required_reputation_faction != 0 {
            let required_rank =
                u32::try_from(sparse_template.required_reputation_rank.max(0)).unwrap_or(0);
            if source
                .reputation_rank(u32::from(sparse_template.required_reputation_faction))
                .unwrap_or(0)
                < required_rank
            {
                return false;
            }
        }
        // C++ `CanUseItem` learning-effect pair (`Player.cpp:11110-11113`): a
        // recipe, mount or pet item whose second effect is already known cannot
        // be used again.
        let effect_spell_ids = source.learning_effects(item_id);
        if let (Some((_, first)), Some((_, second))) =
            (effect_spell_ids.first(), effect_spell_ids.get(1))
            && matches!(*first, 483 | 55_884)
            && i32::try_from(*second)
                .ok()
                .is_some_and(|spell_id| source.knows_spell(spell_id))
        {
            return false;
        }

        let player_class_mask = source.class_mask(source.player_class());
        if sparse_template.allowable_class != 0
            && (u32::try_from(sparse_template.allowable_class).unwrap_or(0) & player_class_mask)
                == 0
        {
            return false;
        }

        let flags2 = sparse_template.flags[1];
        let flags3 = sparse_template.flags[2];
        if (flags2 & ItemFlags2::NoSourceForItemVisual as u32) != 0 {
            return false;
        }
        let Some(quality) = source.item_quality(item_id) else {
            return false;
        };
        if quality == ItemQuality::Artifact as i8 {
            return false;
        }

        match template.class_id {
            ItemClass::Weapon => {
                let subclass = u32::from(item_record);
                if subclass >= 32 {
                    return false;
                }
                // C++ `CollectionMgr::CanAddAppearance` reads the learned
                // `Player::GetWeaponProficiency` mask, not the class default
                // the client receives at creation.
                let weapon_proficiency = source.weapon_proficiency().unwrap_or(0);
                if (weapon_proficiency & (1_u32 << subclass)) == 0 {
                    return false;
                }
                if matches!(
                    subclass,
                    x if x == ItemSubClassWeapon::Exotic as u32
                        || x == ItemSubClassWeapon::Exotic2 as u32
                        || x == ItemSubClassWeapon::Miscellaneous as u32
                        || x == ItemSubClassWeapon::Thrown as u32
                        || x == ItemSubClassWeapon::Spear as u32
                        || x == ItemSubClassWeapon::FishingPole as u32
                ) {
                    return false;
                }
            }
            ItemClass::Armor => {
                let subclass = u32::from(item_record);
                match template.inventory_type {
                    InventoryType::Body
                    | InventoryType::Shield
                    | InventoryType::Cloak
                    | InventoryType::Tabard
                    | InventoryType::Holdable => {}
                    InventoryType::Head
                    | InventoryType::Shoulders
                    | InventoryType::Chest
                    | InventoryType::Waist
                    | InventoryType::Legs
                    | InventoryType::Feet
                    | InventoryType::Wrists
                    | InventoryType::Hands
                    | InventoryType::Robe => {
                        if subclass == ItemSubClassArmor::Miscellaneous as u32 {
                            return false;
                        }
                    }
                    _ => return false,
                }

                if template.inventory_type != InventoryType::Cloak
                    && (source.armor_class_mask(subclass) & player_class_mask) == 0
                {
                    return false;
                }
            }
            _ => return false,
        }

        if quality < ItemQuality::Uncommon as i8
            && ((flags2 & ItemFlags2::IgnoreQualityForItemVisualSource as u32) == 0
                || (flags3 & ItemFlags3::ActsAsTransmogHiddenVisualOption as u32) == 0)
        {
            return false;
        }

        source
            .permanent_appearance_exists(item_modified_appearance_id)
            .is_some_and(|permanent| !permanent)
    }
}

impl PlayerCollectionStateLikeCpp {
    pub fn appearance_class_mask(class_id: u8) -> u32 {
        if class_id == 0 || class_id > 32 {
            0
        } else {
            1_u32 << u32::from(class_id - 1)
        }
    }

    pub fn appearance_armor_class_mask(subclass: u32) -> u32 {
        match subclass {
            x if x == ItemSubClassArmor::Miscellaneous as u32 => 0x0FFF,
            x if x == ItemSubClassArmor::Cloth as u32 => {
                (1 << (5 - 1)) | (1 << (8 - 1)) | (1 << (9 - 1))
            }
            x if x == ItemSubClassArmor::Leather as u32 => {
                (1 << (4 - 1)) | (1 << (10 - 1)) | (1 << (11 - 1)) | (1 << (12 - 1))
            }
            x if x == ItemSubClassArmor::Mail as u32 => (1 << (3 - 1)) | (1 << (7 - 1)),
            x if x == ItemSubClassArmor::Plate as u32 => {
                (1 << (1 - 1)) | (1 << (2 - 1)) | (1 << (6 - 1))
            }
            x if x == ItemSubClassArmor::Cosmetic as u32 => 0x0FFF,
            x if x == ItemSubClassArmor::Shield as u32 => {
                (1 << (1 - 1)) | (1 << (2 - 1)) | (1 << (7 - 1))
            }
            x if x == ItemSubClassArmor::Libram as u32 => 1 << (2 - 1),
            x if x == ItemSubClassArmor::Idol as u32 => 1 << (11 - 1),
            x if x == ItemSubClassArmor::Totem as u32 => 1 << (7 - 1),
            x if x == ItemSubClassArmor::Sigil as u32 => 1 << (6 - 1),
            x if x == ItemSubClassArmor::Relic as u32 => {
                (1 << (2 - 1)) | (1 << (6 - 1)) | (1 << (7 - 1)) | (1 << (11 - 1))
            }
            _ => 0,
        }
    }
}
