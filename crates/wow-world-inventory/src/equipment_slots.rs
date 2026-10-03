// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::collections::BTreeMap;
use wow_constants::WeaponAttackType;
use wow_data::SpellEquippedItemsEntry;
use wow_entities::{EQUIPMENT_SLOT_MAINHAND, EQUIPMENT_SLOT_OFFHAND, EQUIPMENT_SLOT_RANGED};
use wow_world_core::session::{HubRef, PlayerStatsAccessLikeCpp};

/// C++ `SPELL_SCHOOL_MASK_NORMAL` (`SharedDefines.h:329`): the physical school
/// bit `Unit::UpdateDamagePctDoneMods` filters the damage-percent aura by.
const SPELL_SCHOOL_MASK_NORMAL_LIKE_CPP: i32 = 1;
/// C++ `ITEM_ENCHANTMENT_TYPE_DAMAGE` (`DBCEnums.h:965`).
const ITEM_ENCHANTMENT_TYPE_DAMAGE_LIKE_CPP: u8 = 2;
/// C++ `ITEM_ENCHANTMENT_TYPE_TOTEM` (`DBCEnums.h:969`).
const ITEM_ENCHANTMENT_TYPE_TOTEM_LIKE_CPP: u8 = 6;

impl crate::InventoryState {
    /// C++ `Player::HasItemFitToSpellRequirements` (`Player.cpp:24641-24708`)
    /// includes ignore/shield handling that this represented check does not
    /// model. There is no direct World caller: Inventory's
    /// `items::represented_has_item_fit_to_spell_requirements_like_cpp` invokes
    /// this helper from the login passive-equipment path. This does not assert
    /// that the absent branch rejects that caller.
    pub(crate) fn represented_equipped_item_in_slot_fits_spell_requirements_like_cpp(
        &self,
        hub: HubRef<'_>,
        slot: u8,
        equipped: &SpellEquippedItemsEntry,
    ) -> bool {
        let access = hub.core.owned_inventory_access_like_cpp();
        self.resolved_inventory_item_with_access_like_cpp(&access, slot)
            .and_then(|item| {
                self.resolved_inventory_item_object_with_access_like_cpp(&access, item.guid)
            })
            .is_some_and(|item| {
                item.container_guid().is_empty()
                    && item.slot() == slot
                    && hub
                        .catalogs
                        .represented_item_fits_spell_requirements_like_cpp(
                            item.object().entry(),
                            equipped,
                        )
            })
    }

    /// C++ `Player::GetWeaponForAttack(attack, true)` (`Player.cpp:9243-9273`)
    /// selects MAINHAND for BASE and RANGED attacks, and OFFHAND for OFF; it
    /// requires a weapon-class item, applies `IsRanged` for RANGED, and rejects
    /// broken items. This represented selector uses the
    /// RANGED equipment slot for RANGED and checks only that the item is not
    /// broken. C++ `Player::GetUseableItem` (`Player.cpp:9199-9209`) also checks
    /// `CanUseAttackType`, which this helper does not.
    fn represented_usable_weapon_item_id_like_cpp(
        &self,
        access: &PlayerStatsAccessLikeCpp<'_>,
        attack: WeaponAttackType,
    ) -> Option<u32> {
        let slot = match attack {
            WeaponAttackType::BaseAttack => EQUIPMENT_SLOT_MAINHAND,
            WeaponAttackType::OffAttack => EQUIPMENT_SLOT_OFFHAND,
            WeaponAttackType::RangedAttack => EQUIPMENT_SLOT_RANGED,
            WeaponAttackType::Max => return None,
        };
        let inventory_access = access.owned_inventory_access_like_cpp();
        let item = self
            .resolved_inventory_item_with_access_like_cpp(&inventory_access, slot)?;
        self.resolved_inventory_item_object_with_access_like_cpp(&inventory_access, item.guid)
            .is_some_and(|object| !object.is_broken())
            .then_some(item.entry_id)
    }

    /// C++ `Unit::UpdateDamagePctDoneMods` (`Unit.cpp:9033-9072`) multiplies
    /// the `UNIT_MOD_DAMAGE_*` `TOTAL_PCT` by matching
    /// `SPELL_AURA_MOD_DAMAGE_PERCENT_DONE` effects and, for offhand, by the raw
    /// `GetTotalAuraModifier(SPELL_AURA_MOD_OFFHAND_DAMAGE_PCT, ...)` sum.
    /// `Unit::GetTotalAuraModifier` (`Unit.cpp:4818-4844`) returns 0 when no
    /// matching aura is active. This represented calculation keeps the 0.5
    /// offhand base and omits that aura sum. This is an inherited F6 difference;
    /// this move does not change its behavior.
    pub fn represented_weapon_damage_pct_like_cpp(
        &self,
        access: &PlayerStatsAccessLikeCpp<'_>,
    ) -> [f32; 3] {
        let effects = access
            .resolved_aura_effects_with_spell_and_misc_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_DONE,
            )
            .unwrap_or_default();
        std::array::from_fn(|index| {
            let attack =
                <wow_constants::WeaponAttackType as num_traits::FromPrimitive>::from_usize(index)
                    .unwrap_or(wow_constants::WeaponAttackType::BaseAttack);
            let base = match attack {
                wow_constants::WeaponAttackType::OffAttack => 0.5_f32,
                _ => 1.0_f32,
            };
            let weapon_item_id = self.represented_usable_weapon_item_id_like_cpp(access, attack);
            base * effects
                .iter()
                .filter(|(spell_id, misc_value, _)| {
                    misc_value & SPELL_SCHOOL_MASK_NORMAL_LIKE_CPP != 0
                        && access.represented_aura_spell_fits_weapon_like_cpp(
                            *spell_id,
                            weapon_item_id,
                        )
                })
                .fold(1.0_f32, |acc, (_, _, amount)| {
                    acc * (1.0 + *amount as f32 / 100.0)
                })
        })
    }

    /// C++ `Player::UpdateDamageDoneMods` (`Player.cpp:4965-5015`), reached from
    /// `HandleModDamageDone` (`SpellAuraEffects.cpp:4497-4505`) through
    /// `Unit::UpdateAllDamageDoneMods`: the `UNIT_MOD_DAMAGE_*` `TOTAL_VALUE` is
    /// the sum of every active `SPELL_AURA_MOD_DAMAGE_DONE` (13) effect that
    /// covers `SPELL_SCHOOL_MASK_NORMAL` and fits the attack's weapon, plus the
    /// weapon-enchantment `ITEM_ENCHANTMENT_TYPE_DAMAGE`/`TOTEM` term.
    pub fn represented_weapon_damage_flat_like_cpp(
        &self,
        access: &PlayerStatsAccessLikeCpp<'_>,
    ) -> [f32; 3] {
        let effects = access
            .resolved_aura_effects_with_spell_and_misc_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE,
            )
            .unwrap_or_default();
        std::array::from_fn(|index| {
            let attack =
                <wow_constants::WeaponAttackType as num_traits::FromPrimitive>::from_usize(index)
                    .unwrap_or(wow_constants::WeaponAttackType::BaseAttack);
            let weapon_item_id = self.represented_usable_weapon_item_id_like_cpp(access, attack);
            let aura_sum = effects
                .iter()
                .filter(|(spell_id, misc_value, _)| {
                    misc_value & SPELL_SCHOOL_MASK_NORMAL_LIKE_CPP != 0
                        && access.represented_aura_spell_fits_weapon_like_cpp(
                            *spell_id,
                            weapon_item_id,
                        )
                })
                .map(|(_, _, amount)| *amount)
                .sum::<i32>();
            aura_sum as f32 + self.represented_weapon_enchant_damage_like_cpp(access, attack)
        })
    }

    /// C++ `Player::UpdateDamageDoneMods`'s enchantment loop
    /// (`Player.cpp:4991-5015`): for the attack's weapon, every enchantment
    /// slot's `ITEM_ENCHANTMENT_TYPE_DAMAGE` (2) adds
    /// `SpellItemEnchantment::EffectScalingPoints`, and
    /// `ITEM_ENCHANTMENT_TYPE_TOTEM` (6) adds the same scaled by the weapon
    /// delay for shamans only.
    fn represented_weapon_enchant_damage_like_cpp(
        &self,
        access: &PlayerStatsAccessLikeCpp<'_>,
        attack: WeaponAttackType,
    ) -> f32 {
        let Some(item_id) = self.represented_usable_weapon_item_id_like_cpp(access, attack) else {
            return 0.0;
        };
        let slot = match attack {
            WeaponAttackType::BaseAttack => EQUIPMENT_SLOT_MAINHAND,
            WeaponAttackType::OffAttack => EQUIPMENT_SLOT_OFFHAND,
            WeaponAttackType::RangedAttack => EQUIPMENT_SLOT_RANGED,
            WeaponAttackType::Max => return 0.0,
        };
        let inventory_access = access.owned_inventory_access_like_cpp();
        let Some(inventory_item) = self
            .resolved_inventory_item_with_access_like_cpp(&inventory_access, slot)
        else {
            return 0.0;
        };
        let Some(item) = self
            .resolved_inventory_item_object_with_access_like_cpp(
                &inventory_access,
                inventory_item.guid,
            )
        else {
            return 0.0;
        };
        let Some(enchantment_store) = access.spell_item_enchantment_store_like_cpp() else {
            return 0.0;
        };
        let delay_seconds = access.represented_weapon_delay_seconds_like_cpp(item_id);
        let is_shaman = access.player_class_like_cpp() == 7;
        item.data()
            .enchantments
            .iter()
            .filter_map(|enchantment| u32::try_from(enchantment.id).ok())
            .filter_map(|enchantment_id| enchantment_store.get(enchantment_id))
            .flat_map(|entry| {
                entry
                    .effect
                    .iter()
                    .zip(entry.effect_scaling_points.iter())
                    .map(move |(effect, points)| match *effect {
                        ITEM_ENCHANTMENT_TYPE_DAMAGE_LIKE_CPP => *points,
                        ITEM_ENCHANTMENT_TYPE_TOTEM_LIKE_CPP if is_shaman => {
                            *points * delay_seconds
                        }
                        _ => 0.0,
                    })
            })
            .sum()
    }

    /// C++ `Player::UpdateWeaponDependentCritAuras` (`Player.cpp:8079-8107`):
    /// the `SPELL_AURA_MOD_WEAPON_CRIT_PERCENT` sum filtered by
    /// `CheckAttackFitToAuraRequirement` (`Player.cpp:8145-8156`) for the
    /// attack's weapon, plus the unfiltered `SPELL_AURA_MOD_CRIT_PCT` sum. C++
    /// stores the result per attack as the `FLAT_MOD` critical base value.
    pub fn represented_weapon_crit_aura_modifier_like_cpp(
        &self,
        access: &PlayerStatsAccessLikeCpp<'_>,
        attack: WeaponAttackType,
    ) -> f32 {
        let weapon_item_id = self.represented_usable_weapon_item_id_like_cpp(access, attack);
        let weapon_dependent = access
            .resolved_aura_effect_amounts_by_spell_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_MOD_WEAPON_CRIT_PERCENT,
            )
            .unwrap_or_default()
            .into_iter()
            .filter(|(spell_id, _)| {
                access.represented_aura_spell_fits_weapon_like_cpp(*spell_id, weapon_item_id)
            })
            .map(|(_, amount)| amount)
            .sum::<i32>();
        let global = access
            .resolved_aura_effect_amounts_by_spell_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_MOD_CRIT_PCT,
            )
            .unwrap_or_default()
            .into_iter()
            .map(|(_, amount)| amount)
            .sum::<i32>();
        (weapon_dependent + global) as f32
    }

    /// C++ `Player::UpdateExpertise`'s
    /// `GetTotalAuraModifier(SPELL_AURA_MOD_EXPERTISE, predicate)`
    /// (`StatSystem.cpp:767-770`): sum of every active
    /// `SPELL_AURA_MOD_EXPERTISE` effect whose spell is fit for the weapon of
    /// `attack`, with the `SPELL_GROUP_STACK_RULE_EXCLUSIVE_SAME_EFFECT`
    /// groups folded to their highest absolute amount
    /// (`Unit.cpp:4818-4850`). C++ writes the result per attack, so the
    /// mainhand and offhand weapons can select different auras.
    pub fn represented_expertise_aura_modifier_like_cpp(
        &self,
        access: &PlayerStatsAccessLikeCpp<'_>,
        attack: WeaponAttackType,
    ) -> i32 {
        let weapon_item_id = self.represented_usable_weapon_item_id_like_cpp(access, attack);
        let Some(effects) = access.resolved_aura_effect_amounts_by_spell_like_cpp(
            wow_data::spell::aura_types::SPELL_AURA_MOD_EXPERTISE,
        ) else {
            return 0;
        };

        let mut same_effect_groups: BTreeMap<u32, i32> = BTreeMap::new();
        let mut modifier = 0;
        for (spell_id, amount) in effects {
            if !access.represented_aura_spell_fits_weapon_like_cpp(spell_id, weapon_item_id) {
                continue;
            }
            // A spell belongs to at most one same-effect group per aura type.
            let same_effect_group = access
                .spell_spell_group_map_bounds_like_cpp(spell_id as u32)
                .iter()
                .copied()
                .find(|group_id| {
                    access
                        .same_effect_stack_rule_aura_types_like_cpp(*group_id)
                        .is_some_and(|aura_types| {
                            aura_types
                                .contains(&wow_data::spell::aura_types::SPELL_AURA_MOD_EXPERTISE)
                        })
                });
            if let Some(group_id) = same_effect_group {
                same_effect_groups
                    .entry(group_id)
                    .and_modify(|current| {
                        if current.unsigned_abs() < amount.unsigned_abs() {
                            *current = amount;
                        }
                    })
                    .or_insert(amount);
            } else {
                modifier += amount;
            }
        }
        modifier + same_effect_groups.values().sum::<i32>()
    }
}
