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

    /// Retained projection: lend the bonus record to the enchantment and
    /// equipment rules that write it.
    ///
    /// C++ applies those rules while holding the Player
    /// (`Player::_ApplyItemBonuses`, `Player::ApplyEnchantment`), and RustyCore
    /// keeps them in `wow-world` because they walk catalog-shaped actions that
    /// may not enter `wow-entities`. **Exit condition:** the borrow retires when
    /// the enchantment/equipment application contract moves behind a named
    /// operation that takes the resolved effect instead of the record.
    pub fn with_bonuses_mut_like_cpp<R>(
        &mut self,
        apply: impl FnOnce(&mut PlayerItemBonusStateLikeCpp) -> R,
    ) -> R {
        apply(&mut self.bonuses)
    }
}
