// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical Player item-modifier runtime: equipment bonuses, item-set effects
//! and the item level limits.
//!
//! C++ spreads these across the Player: `_ApplyItemBonuses` accumulates the
//! equipment contribution, `Player::ItemSetEff` holds one `ItemSetEffect`
//! (`Entities/Item/Item.h:41`) per active set, and the item level limits live in
//! the player's update fields, read back by `Item::GetItemLevel(Player const*)`.
//! The set transitions are the free functions `AddItemsSetItem`
//! (`Entities/Item/Item.cpp:57`) and `RemoveItemsSetItem` (`:146`), which C++
//! declares on the Player boundary (`Player.h:3160-3161`).
//!
//! Separated from `player_gameplay_state.rs` under #769, which also closed the
//! container's members: the three records stay value-shaped, and the container
//! only changes through named operations.

use std::collections::{BTreeSet, HashMap, HashSet};

use super::{
    ApplyEnchantmentBaseMod, ApplyEnchantmentCombatRating, ApplyEnchantmentEffectAction,
    ApplyEnchantmentUnitMod, ApplyEnchantmentUnitModifier,
};
use wow_constants::{Stats, WeaponAttackType};
use wow_core::ObjectGuid;

/// Canonical runtime accumulated by C++ `Player::_ApplyItemBonuses`.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerItemBonusStateLikeCpp {
    pub mana_base: i32,
    pub health_base: i32,
    pub armor_base: i32,
    pub armor_total: i32,
    pub stats_base: [i32; 5],
    pub attack_power_total: i32,
    pub ranged_attack_power_total: i32,
    pub resistances_base: [i32; 7],
    pub combat_ratings: [i32; 32],
    pub mana_regen_bonus: i32,
    pub spell_power_bonus: i32,
    pub health_regen_bonus: i32,
    pub spell_penetration_bonus: i32,
    pub shield_block_base_mod: i32,
    pub shield_block_value: u32,
    pub weapon_damage: [[f32; 2]; 3],
    pub base_attack_time: [u32; 3],
    pub stat_buff_updates: Vec<Stats>,
    pub damage_physical_updates: Vec<WeaponAttackType>,
}

impl Default for PlayerItemBonusStateLikeCpp {
    fn default() -> Self {
        Self {
            mana_base: 0,
            health_base: 0,
            armor_base: 0,
            armor_total: 0,
            stats_base: [0; 5],
            attack_power_total: 0,
            ranged_attack_power_total: 0,
            resistances_base: [0; 7],
            combat_ratings: [0; 32],
            mana_regen_bonus: 0,
            spell_power_bonus: 0,
            health_regen_bonus: 0,
            spell_penetration_bonus: 0,
            shield_block_base_mod: 0,
            shield_block_value: 0,
            weapon_damage: [[0.0; 2]; 3],
            base_attack_time: [0; 3],
            stat_buff_updates: Vec::new(),
            damage_physical_updates: Vec::new(),
        }
    }
}

/// Canonical C++ `ItemSetEffect` projection. DB2 row IDs replace pointers.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlayerItemSetEffectLikeCpp {
    pub item_set_id: u32,
    pub equipped_items: HashSet<ObjectGuid>,
    pub set_bonuses: BTreeSet<u32>,
}

/// Player-owned item level limits consumed by `Item::GetItemLevel(Player const*)`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PlayerItemLevelCapsLikeCpp {
    pub min_item_level_cutoff: u32,
    pub min_item_level: u32,
    pub max_item_level: u32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlayerItemModifierRuntimeStateLikeCpp {
    bonuses: PlayerItemBonusStateLikeCpp,
    item_set_effects: HashMap<u32, PlayerItemSetEffectLikeCpp>,
    item_level_caps: PlayerItemLevelCapsLikeCpp,
}

impl PlayerItemModifierRuntimeStateLikeCpp {
    // ---- reads -------------------------------------------------------------

    /// The accumulated equipment bonus record.
    #[must_use]
    pub fn bonuses_like_cpp(&self) -> &PlayerItemBonusStateLikeCpp {
        &self.bonuses
    }

    /// Copy the bonus record, for a caller that must own it.
    #[must_use]
    pub fn bonuses_snapshot_like_cpp(&self) -> PlayerItemBonusStateLikeCpp {
        self.bonuses.clone()
    }

    /// The active item-set effects, keyed by `ItemSetID` as C++ matches them in
    /// `Player::ItemSetEff`.
    #[must_use]
    pub fn item_set_effects_like_cpp(&self) -> &HashMap<u32, PlayerItemSetEffectLikeCpp> {
        &self.item_set_effects
    }

    /// One active item-set effect, as C++ finds it by `ItemSetID`
    /// (`Item.cpp:90`, `:157`).
    #[must_use]
    pub fn item_set_effect_like_cpp(
        &self,
        item_set_id: u32,
    ) -> Option<&PlayerItemSetEffectLikeCpp> {
        self.item_set_effects.get(&item_set_id)
    }

    /// The item level limits `Item::GetItemLevel(Player const*)` reads back.
    #[must_use]
    pub fn item_level_caps_like_cpp(&self) -> PlayerItemLevelCapsLikeCpp {
        self.item_level_caps
    }

    // ---- transitions -------------------------------------------------------

    /// C++ `AddItemsSetItem` (`Item.cpp:57`) as it reaches the container: the
    /// effect for this set is created on first use and the equipped item joins
    /// it. Answers the equipped count C++ then compares against each set
    /// spell's `Threshold` (`:108`).
    pub fn add_item_set_item_like_cpp(&mut self, item_set_id: u32, item_guid: ObjectGuid) -> usize {
        let effect = self.item_set_effects.entry(item_set_id).or_insert_with(|| {
            PlayerItemSetEffectLikeCpp {
                item_set_id,
                ..Default::default()
            }
        });
        effect.equipped_items.insert(item_guid);
        effect.equipped_items.len()
    }

    /// C++ `AddItemsSetItem` inserting one qualifying set bonus into
    /// `SetBonuses` (`Item.cpp:122`). Answers whether it was newly added, as
    /// C++ learns from the insert before it applies the spell.
    pub fn add_item_set_bonus_like_cpp(&mut self, item_set_id: u32, spell_entry_id: u32) -> bool {
        self.item_set_effects
            .get_mut(&item_set_id)
            .is_some_and(|effect| effect.set_bonuses.insert(spell_entry_id))
    }

    /// C++ `RemoveItemsSetItem` (`Item.cpp:146`) erasing the item from its
    /// effect (`:174`). Answers the remaining equipped count, or `None` when no
    /// effect exists — the case C++ returns early for at `:172`.
    pub fn remove_item_set_item_like_cpp(
        &mut self,
        item_set_id: u32,
        item_guid: ObjectGuid,
    ) -> Option<usize> {
        let effect = self.item_set_effects.get_mut(&item_set_id)?;
        effect.equipped_items.remove(&item_guid);
        Some(effect.equipped_items.len())
    }

    /// C++ `RemoveItemsSetItem` dropping one set bonus that fell below its
    /// threshold (`Item.cpp:188`). Answers whether it was held, as C++ checks
    /// before it removes the spell.
    pub fn remove_item_set_bonus_like_cpp(
        &mut self,
        item_set_id: u32,
        spell_entry_id: u32,
    ) -> bool {
        self.item_set_effects
            .get_mut(&item_set_id)
            .is_some_and(|effect| effect.set_bonuses.remove(&spell_entry_id))
    }

    /// The tail of C++ `RemoveItemsSetItem` (`Item.cpp:192`): once the last
    /// equipped item of a set is gone, its effect is deleted. Kept separate
    /// because the bonus removal between it and the erase needs the set's
    /// catalog rows, which live in `wow-world`.
    pub fn drop_empty_item_set_effect_like_cpp(&mut self, item_set_id: u32) -> bool {
        if self
            .item_set_effects
            .get(&item_set_id)
            .is_some_and(|effect| effect.equipped_items.is_empty())
        {
            self.item_set_effects.remove(&item_set_id);
            return true;
        }
        false
    }

    /// Install the item level limits the client is told about.
    pub fn set_item_level_caps_like_cpp(&mut self, caps: PlayerItemLevelCapsLikeCpp) {
        self.item_level_caps = caps;
    }

    /// Return the equipment bonus record to its unequipped state, as C++ does
    /// by removing every item's bonuses before it reapplies them.
    pub fn reset_bonuses_like_cpp(&mut self) {
        self.bonuses = PlayerItemBonusStateLikeCpp::default();
    }

    /// Apply one resolved enchantment/equipment effect to the Player-owned
    /// bonus state.
    ///
    /// This is the state-only tail of C++ `Player::_ApplyItemBonuses` and
    /// `Player::ApplyEnchantment` (`Player.cpp:7688-7975`,
    /// `Player.cpp:13058-13389`). Catalog lookup, spell casts and aura
    /// publication remain session concerns; unsupported action variants are
    /// deliberately ignored here.
    pub fn apply_enchantment_effect_action_like_cpp(
        &mut self,
        action: ApplyEnchantmentEffectAction,
    ) {
        apply_enchantment_effect_action_to_bonus_state_like_cpp(&mut self.bonuses, action);
    }
}

fn apply_enchantment_effect_action_to_bonus_state_like_cpp(
    state: &mut PlayerItemBonusStateLikeCpp,
    action: ApplyEnchantmentEffectAction,
) {
    match action {
        ApplyEnchantmentEffectAction::UnitModifier {
            unit_mod,
            modifier,
            amount,
            apply,
        } => apply_unit_modifier_like_cpp(state, unit_mod, modifier, amount, apply),
        ApplyEnchantmentEffectAction::UpdateStatBuffMod(stat) => state.stat_buff_updates.push(stat),
        ApplyEnchantmentEffectAction::RatingModifier {
            rating,
            amount,
            apply,
        } => {
            if let Some(index) = combat_rating_index_like_cpp(rating) {
                apply_i32_delta_like_cpp(&mut state.combat_ratings[index], amount, apply);
            }
        }
        ApplyEnchantmentEffectAction::ManaRegenBonus { amount, apply } => {
            apply_i32_delta_like_cpp(&mut state.mana_regen_bonus, amount, apply)
        }
        ApplyEnchantmentEffectAction::SpellPowerBonus { amount, apply } => {
            apply_i32_delta_like_cpp(&mut state.spell_power_bonus, amount, apply)
        }
        ApplyEnchantmentEffectAction::HealthRegenBonus { amount, apply } => {
            apply_i32_delta_like_cpp(&mut state.health_regen_bonus, amount, apply)
        }
        ApplyEnchantmentEffectAction::SpellPenetrationBonus { amount, apply } => {
            apply_i32_delta_like_cpp(&mut state.spell_penetration_bonus, amount, apply)
        }
        ApplyEnchantmentEffectAction::BaseModFlatValue {
            base_mod: ApplyEnchantmentBaseMod::ShieldBlockValue,
            amount,
            apply,
        } => apply_i32_delta_like_cpp(&mut state.shield_block_base_mod, amount, apply),
        ApplyEnchantmentEffectAction::SetShieldBlockValue { amount } => {
            state.shield_block_value = amount
        }
        ApplyEnchantmentEffectAction::SetBaseWeaponDamage {
            attack_type,
            bound,
            amount_bits,
        } => {
            let attack = attack_type as usize;
            if attack < state.weapon_damage.len() {
                let bound = match bound {
                    super::WeaponDamageBoundLikeCpp::Min => 0,
                    super::WeaponDamageBoundLikeCpp::Max => 1,
                };
                state.weapon_damage[attack][bound] = f32::from_bits(amount_bits);
            }
        }
        ApplyEnchantmentEffectAction::SetBaseAttackTime {
            attack_type,
            time_ms,
        } => {
            let attack = attack_type as usize;
            if attack < state.base_attack_time.len() {
                state.base_attack_time[attack] = time_ms;
            }
        }
        ApplyEnchantmentEffectAction::UpdateDamagePhysical { attack_type } => {
            state.damage_physical_updates.push(attack_type)
        }
        ApplyEnchantmentEffectAction::Noop
        | ApplyEnchantmentEffectAction::DeferredCombatSpell
        | ApplyEnchantmentEffectAction::DeferredUseSpell
        | ApplyEnchantmentEffectAction::UpdateDamageDoneMods { .. }
        | ApplyEnchantmentEffectAction::CastEquipSpell { .. }
        | ApplyEnchantmentEffectAction::RemoveEquipSpellAura { .. }
        | ApplyEnchantmentEffectAction::UnhandledStatModifier { .. }
        | ApplyEnchantmentEffectAction::MissingItemTemplateForAttack { .. }
        | ApplyEnchantmentEffectAction::Unknown { .. } => {}
    }
}

fn apply_unit_modifier_like_cpp(
    state: &mut PlayerItemBonusStateLikeCpp,
    unit_mod: ApplyEnchantmentUnitMod,
    modifier: ApplyEnchantmentUnitModifier,
    amount: u32,
    apply: bool,
) {
    match (unit_mod, modifier) {
        (ApplyEnchantmentUnitMod::Mana, ApplyEnchantmentUnitModifier::BaseValue) => {
            apply_i32_delta_like_cpp(&mut state.mana_base, amount, apply)
        }
        (ApplyEnchantmentUnitMod::Health, ApplyEnchantmentUnitModifier::BaseValue) => {
            apply_i32_delta_like_cpp(&mut state.health_base, amount, apply)
        }
        (ApplyEnchantmentUnitMod::Armor, ApplyEnchantmentUnitModifier::BaseValue) => {
            apply_i32_delta_like_cpp(&mut state.armor_base, amount, apply)
        }
        (ApplyEnchantmentUnitMod::Armor, ApplyEnchantmentUnitModifier::TotalValue) => {
            apply_i32_delta_like_cpp(&mut state.armor_total, amount, apply)
        }
        (ApplyEnchantmentUnitMod::AttackPower, ApplyEnchantmentUnitModifier::TotalValue) => {
            apply_i32_delta_like_cpp(&mut state.attack_power_total, amount, apply)
        }
        (ApplyEnchantmentUnitMod::AttackPowerRanged, ApplyEnchantmentUnitModifier::TotalValue) => {
            apply_i32_delta_like_cpp(&mut state.ranged_attack_power_total, amount, apply)
        }
        (ApplyEnchantmentUnitMod::Resistance(school), _) => {
            let school = school as usize;
            if school < state.resistances_base.len() {
                apply_i32_delta_like_cpp(&mut state.resistances_base[school], amount, apply);
            }
        }
        (
            unit_mod,
            ApplyEnchantmentUnitModifier::BaseValue | ApplyEnchantmentUnitModifier::TotalValue,
        ) => {
            if let Some(index) = unit_mod_stat_index_like_cpp(unit_mod) {
                apply_i32_delta_like_cpp(&mut state.stats_base[index], amount, apply);
            }
        }
    }
}

fn apply_i32_delta_like_cpp(target: &mut i32, amount: u32, apply: bool) {
    let amount = i32::try_from(amount).unwrap_or(i32::MAX);
    if apply {
        *target = target.saturating_add(amount);
    } else {
        *target = target.saturating_sub(amount);
    }
}

fn unit_mod_stat_index_like_cpp(unit_mod: ApplyEnchantmentUnitMod) -> Option<usize> {
    match unit_mod {
        ApplyEnchantmentUnitMod::StatStrength => Some(Stats::Strength as usize),
        ApplyEnchantmentUnitMod::StatAgility => Some(Stats::Agility as usize),
        ApplyEnchantmentUnitMod::StatStamina => Some(Stats::Stamina as usize),
        ApplyEnchantmentUnitMod::StatIntellect => Some(Stats::Intellect as usize),
        ApplyEnchantmentUnitMod::StatSpirit => Some(Stats::Spirit as usize),
        _ => None,
    }
}

fn combat_rating_index_like_cpp(rating: ApplyEnchantmentCombatRating) -> Option<usize> {
    match rating {
        ApplyEnchantmentCombatRating::DefenseSkill => Some(1),
        ApplyEnchantmentCombatRating::Dodge => Some(2),
        ApplyEnchantmentCombatRating::Parry => Some(3),
        ApplyEnchantmentCombatRating::Block => Some(4),
        ApplyEnchantmentCombatRating::HitMelee => Some(5),
        ApplyEnchantmentCombatRating::HitRanged => Some(6),
        ApplyEnchantmentCombatRating::HitSpell => Some(7),
        ApplyEnchantmentCombatRating::CritMelee => Some(8),
        ApplyEnchantmentCombatRating::CritRanged => Some(9),
        ApplyEnchantmentCombatRating::CritSpell => Some(10),
        ApplyEnchantmentCombatRating::HasteMelee => Some(17),
        ApplyEnchantmentCombatRating::HasteRanged => Some(18),
        ApplyEnchantmentCombatRating::HasteSpell => Some(19),
        ApplyEnchantmentCombatRating::Expertise => Some(23),
        ApplyEnchantmentCombatRating::ArmorPenetration => Some(24),
    }
}
