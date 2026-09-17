//! Equipment slot resolution and the stats an equipped item contributes.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

/// C++ `SPELL_SCHOOL_MASK_NORMAL` (`SharedDefines.h:329`): the physical school
/// bit `Unit::UpdateDamagePctDoneMods` filters the damage-percent aura by.
const SPELL_SCHOOL_MASK_NORMAL_LIKE_CPP: i32 = 1;

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

    /// C++ `Player::GetWeaponForAttack(attack, true)`
    /// (`Player.cpp:9243-9270`) for the weapon slots that supply
    /// `Player::UpdateExpertise` (`StatSystem.cpp:759-786`): the equipped item
    /// in the mainhand/offhand slot, rejected when it is broken. The
    /// represented equip path already enforces `CanUseItem`, and
    /// `RANGED_ATTACK` returns early in C++.
    fn represented_usable_weapon_item_id_like_cpp(&self, attack: WeaponAttackType) -> Option<u32> {
        let slot = match attack {
            WeaponAttackType::BaseAttack => EQUIPMENT_SLOT_MAINHAND,
            WeaponAttackType::OffAttack => EQUIPMENT_SLOT_OFFHAND,
            WeaponAttackType::RangedAttack | WeaponAttackType::Max => return None,
        };
        let item = self.resolved_inventory_item_like_cpp(slot)?;
        self.resolved_inventory_item_object_like_cpp(item.guid)
            .is_some_and(|object| !object.is_broken())
            .then_some(item.entry_id)
    }

    /// C++ `SpellInfo::IsItemFitToSpellRequirements`
    /// (`SpellInfo.cpp:1757-1768`) as reached from the `UpdateExpertise`
    /// `AuraEffectFilter`: an item-neutral spell (no `SpellEquippedItems` row
    /// or `EquippedItemClass == -1`) always matches; an item-dependent spell
    /// matches only a present weapon whose template fits the class/subclass
    /// mask.
    fn represented_aura_spell_fits_weapon_like_cpp(
        &self,
        spell_id: i32,
        weapon_item_id: Option<u32>,
    ) -> bool {
        let Some(equipped) = self
            .spell_catalogs
            .spell_equipped_items_store
            .as_ref()
            .and_then(|store| store.entry_for_spell_id_like_cpp(spell_id))
        else {
            return true;
        };
        if equipped.equipped_item_class < 0 {
            return true;
        }
        weapon_item_id.is_some_and(|item_id| {
            self.represented_item_fits_spell_requirements_like_cpp(item_id, equipped)
        })
    }

    /// C++ `Unit::UpdateDamagePctDoneMods` (`Unit.cpp:9033-9072`), reached from
    /// `Player::UpdateWeaponDependentAuras` on equip and login: the
    /// `UNIT_MOD_DAMAGE_*` `TOTAL_PCT` is the C++ base factor (mainhand and
    /// ranged 1.0, offhand 0.5) multiplied by every active
    /// `SPELL_AURA_MOD_DAMAGE_PERCENT_DONE` effect that covers
    /// `SPELL_SCHOOL_MASK_NORMAL` and fits the attack's weapon.
    ///
    /// Deliberate departure: the source then multiplies the offhand factor by
    /// `GetTotalAuraModifier(SPELL_AURA_MOD_OFFHAND_DAMAGE_PCT, ...)`, a raw sum
    /// that is 0 when no such aura is active and therefore zeroes offhand
    /// damage on every equip/login. RustyCore keeps the evident intent (the 0.5
    /// base times the physical multiplier) and does not apply that term;
    /// `SPELL_AURA_MOD_OFFHAND_DAMAGE_PCT` remains a separate gate that needs
    /// capture evidence for its scale. Ranged weapon requirements are also
    /// excluded rather than resolved because the represented
    /// `GetWeaponForAttack` helper covers the melee slots only.
    pub(crate) fn represented_weapon_damage_pct_like_cpp(&self) -> [f32; 3] {
        let effects = self
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
            let weapon_item_id = self.represented_usable_weapon_item_id_like_cpp(attack);
            base * effects
                .iter()
                .filter(|(spell_id, misc_value, _)| {
                    misc_value & SPELL_SCHOOL_MASK_NORMAL_LIKE_CPP != 0
                        && self
                            .represented_aura_spell_fits_weapon_like_cpp(*spell_id, weapon_item_id)
                })
                .fold(1.0_f32, |acc, (_, _, amount)| {
                    acc * (1.0 + *amount as f32 / 100.0)
                })
        })
    }

    /// C++ `Player::UpdateWeaponDependentCritAuras` (`Player.cpp:8079-8107`):
    /// the `SPELL_AURA_MOD_WEAPON_CRIT_PERCENT` sum filtered by
    /// `CheckAttackFitToAuraRequirement` (`Player.cpp:8145-8156`) for the
    /// attack's weapon, plus the unfiltered `SPELL_AURA_MOD_CRIT_PCT` sum. C++
    /// stores the result per attack as the `FLAT_MOD` critical base value.
    pub(crate) fn represented_weapon_crit_aura_modifier_like_cpp(
        &self,
        attack: WeaponAttackType,
    ) -> f32 {
        let weapon_item_id = self.represented_usable_weapon_item_id_like_cpp(attack);
        let weapon_dependent = self
            .resolved_aura_effect_amounts_by_spell_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_MOD_WEAPON_CRIT_PERCENT,
            )
            .unwrap_or_default()
            .into_iter()
            .filter(|(spell_id, _)| {
                self.represented_aura_spell_fits_weapon_like_cpp(*spell_id, weapon_item_id)
            })
            .map(|(_, amount)| amount)
            .sum::<i32>();
        let global = self
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
    pub(crate) fn represented_expertise_aura_modifier_like_cpp(
        &self,
        attack: WeaponAttackType,
    ) -> i32 {
        let weapon_item_id = self.represented_usable_weapon_item_id_like_cpp(attack);
        let Some(effects) = self.resolved_aura_effect_amounts_by_spell_like_cpp(
            wow_data::spell::aura_types::SPELL_AURA_MOD_EXPERTISE,
        ) else {
            return 0;
        };

        let mut same_effect_groups: BTreeMap<u32, i32> = BTreeMap::new();
        let mut modifier = 0;
        for (spell_id, amount) in effects {
            if !self.represented_aura_spell_fits_weapon_like_cpp(spell_id, weapon_item_id) {
                continue;
            }
            // A spell belongs to at most one same-effect group per aura type.
            let same_effect_group = self
                .spell_spell_group_map_bounds_like_cpp(spell_id as u32)
                .iter()
                .copied()
                .find(|group_id| {
                    self.same_effect_stack_rule_aura_types_like_cpp(*group_id)
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
