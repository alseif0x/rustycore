//! Equipment slot resolution and the stats an equipped item contributes.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

impl WorldSession {
    pub(in crate::session) fn represented_equipped_item_in_slot_fits_spell_requirements_like_cpp(
        &self,
        slot: u8,
        equipped: &SpellEquippedItemsEntry,
    ) -> bool {
        self.resolved_inventory_item_objects_like_cpp()
            .is_some_and(|items| {
                items
                    .values()
                    .find(|item| item.container_guid().is_empty() && item.slot() == slot)
                    .is_some_and(|item| {
                        self.represented_item_fits_spell_requirements_like_cpp(
                            item.object().entry(),
                            equipped,
                        )
                    })
            })
    }
    pub(in crate::session) fn represented_total_avg_equipment_slot_candidates_like_cpp(
        inventory_type: InventoryType,
        can_dual_wield: bool,
        can_titan_grip: bool,
    ) -> Vec<(u8, bool)> {
        match inventory_type {
            InventoryType::Head => vec![(EQUIPMENT_SLOT_HEAD, false)],
            InventoryType::Neck => vec![(EQUIPMENT_SLOT_NECK, false)],
            InventoryType::Shoulders => vec![(EQUIPMENT_SLOT_SHOULDERS, false)],
            InventoryType::Body => vec![(EQUIPMENT_SLOT_BODY, false)],
            InventoryType::Robe | InventoryType::Chest => vec![(EQUIPMENT_SLOT_CHEST, false)],
            InventoryType::Waist => vec![(EQUIPMENT_SLOT_WAIST, false)],
            InventoryType::Legs => vec![(EQUIPMENT_SLOT_LEGS, false)],
            InventoryType::Feet => vec![(EQUIPMENT_SLOT_FEET, false)],
            InventoryType::Wrists => vec![(EQUIPMENT_SLOT_WRISTS, false)],
            InventoryType::Hands => vec![(EQUIPMENT_SLOT_HANDS, false)],
            InventoryType::Cloak => vec![(EQUIPMENT_SLOT_BACK, false)],
            InventoryType::Finger => {
                vec![
                    (EQUIPMENT_SLOT_FINGER1, false),
                    (EQUIPMENT_SLOT_FINGER2, true),
                ]
            }
            InventoryType::Trinket => {
                vec![
                    (EQUIPMENT_SLOT_TRINKET1, false),
                    (EQUIPMENT_SLOT_TRINKET2, true),
                ]
            }
            InventoryType::Weapon => {
                let mut slots = vec![(EQUIPMENT_SLOT_MAINHAND, false)];
                if can_dual_wield {
                    slots.push((EQUIPMENT_SLOT_OFFHAND, true));
                }
                slots
            }
            InventoryType::Weapon2Hand => {
                let mut slots = vec![(EQUIPMENT_SLOT_MAINHAND, false)];
                if can_dual_wield && can_titan_grip {
                    slots.push((EQUIPMENT_SLOT_OFFHAND, true));
                }
                slots
            }
            InventoryType::Ranged | InventoryType::RangedRight | InventoryType::WeaponMainhand => {
                vec![(EQUIPMENT_SLOT_MAINHAND, false)]
            }
            InventoryType::Shield | InventoryType::Holdable | InventoryType::WeaponOffhand => {
                vec![(EQUIPMENT_SLOT_OFFHAND, false)]
            }
            InventoryType::NonEquip
            | InventoryType::Bag
            | InventoryType::Tabard
            | InventoryType::Ammo
            | InventoryType::Thrown
            | InventoryType::Quiver
            | InventoryType::Relic
            | InventoryType::ProfessionTool
            | InventoryType::ProfessionGear
            | InventoryType::EquipableSpellOffensive
            | InventoryType::EquipableSpellUtility
            | InventoryType::EquipableSpellDefensive
            | InventoryType::EquipableSpellMobility => Vec::new(),
        }
    }
}
