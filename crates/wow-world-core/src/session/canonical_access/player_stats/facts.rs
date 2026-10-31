// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Borrowed canonical Player inputs for the shared character-stats planner.

use wow_entities::{
    EQUIPMENT_SLOT_MAINHAND, EQUIPMENT_SLOT_OFFHAND, EQUIPMENT_SLOT_RANGED, Player,
    PlayerItemBonusStateLikeCpp,
};

use super::{PlayerStatAuraCatalogs, PlayerStatAuraView};

/// The selected weapon data consumed by the existing Inventory stats planner.
///
/// Enchantment IDs are copied from the fixed item enchantment slots only; the
/// Player and Item objects remain owned by their existing canonical stores.
pub struct PlayerStatWeaponFacts {
    entry_id: u32,
    enchantment_ids: Vec<u32>,
}

impl PlayerStatWeaponFacts {
    /// Build a selected weapon input for explicit planner fixtures.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn from_parts(entry_id: u32, enchantment_ids: Vec<u32>) -> Self {
        Self {
            entry_id,
            enchantment_ids,
        }
    }

    pub fn entry_id(&self) -> u32 {
        self.entry_id
    }

    pub fn enchantment_ids(&self) -> &[u32] {
        &self.enchantment_ids
    }
}

/// Immutable selected inputs borrowed from one canonical Player instance.
///
/// This view contains no Player/Session handle and does no owner lookup. The
/// caller must already hold the canonical Player borrow for the whole use.
pub struct PlayerStatFacts<'a> {
    race: u8,
    class: u8,
    level: u8,
    can_parry: bool,
    can_block: bool,
    form_id: u32,
    item_bonuses: &'a PlayerItemBonusStateLikeCpp,
    aura_view: PlayerStatAuraView<'a>,
    previous_weapon_damage_pct: [f32; 3],
    main_hand: Option<PlayerStatWeaponFacts>,
    off_hand: Option<PlayerStatWeaponFacts>,
    ranged: Option<PlayerStatWeaponFacts>,
}

impl<'a> PlayerStatFacts<'a> {
    /// Select the minimal inputs from an already borrowed Player.
    ///
    /// Race, class and level must be hydrated. Missing or broken weapons retain
    /// the existing Inventory selector's `None` meaning. The represented
    /// ranged selector continues to read the RANGED equipment slot.
    pub fn from_player(
        player: &'a Player,
        aura_catalogs: PlayerStatAuraCatalogs<'a>,
    ) -> Option<Self> {
        let race = player.race_like_cpp();
        let class = player.class_like_cpp();
        let level = player.level_like_cpp();
        if race == 0 || class == 0 || level == 0 {
            return None;
        }

        let unit = player.unit();
        let aura_view = PlayerStatAuraView::new(
            unit.subsystems().auras.runtime_applications_like_cpp(),
            aura_catalogs,
        )?;
        let item_bonuses = player.gameplay_state().item_modifiers.bonuses_like_cpp();
        let inventory = player.inventory_runtime_like_cpp();

        Some(Self {
            race,
            class,
            level,
            can_parry: unit.can_parry_like_cpp(),
            can_block: unit.can_block_like_cpp(),
            form_id: {
                let form_id = unit.shapeshift_form_id_like_cpp();
                if form_id != 0 {
                    u32::from(form_id)
                } else {
                    player.shapeshift_form_id_like_cpp()
                }
            },
            item_bonuses,
            aura_view,
            previous_weapon_damage_pct: player.effective_combat_stats_like_cpp().weapon_damage_pct,
            main_hand: weapon_facts(inventory, EQUIPMENT_SLOT_MAINHAND),
            off_hand: weapon_facts(inventory, EQUIPMENT_SLOT_OFFHAND),
            ranged: weapon_facts(inventory, EQUIPMENT_SLOT_RANGED),
        })
    }

    /// Build borrowed planner inputs for explicit fixtures without a mutable
    /// Player mirror. An explicit projection request may supply identity even
    /// when the fixture's stored identity is unset.
    #[cfg(any(test, feature = "test-fixtures"))]
    #[allow(clippy::too_many_arguments)]
    pub fn from_parts(
        race: u8,
        class: u8,
        level: u8,
        can_parry: bool,
        can_block: bool,
        form_id: u32,
        item_bonuses: &'a PlayerItemBonusStateLikeCpp,
        aura_view: PlayerStatAuraView<'a>,
        previous_weapon_damage_pct: [f32; 3],
        main_hand: Option<PlayerStatWeaponFacts>,
        off_hand: Option<PlayerStatWeaponFacts>,
        ranged: Option<PlayerStatWeaponFacts>,
    ) -> Self {
        Self {
            race,
            class,
            level,
            can_parry,
            can_block,
            form_id,
            item_bonuses,
            aura_view,
            previous_weapon_damage_pct,
            main_hand,
            off_hand,
            ranged,
        }
    }

    pub fn race(&self) -> u8 {
        self.race
    }

    pub fn class(&self) -> u8 {
        self.class
    }

    pub fn level(&self) -> u8 {
        self.level
    }

    pub fn can_parry(&self) -> bool {
        self.can_parry
    }

    pub fn can_block(&self) -> bool {
        self.can_block
    }

    pub fn form_id(&self) -> u32 {
        self.form_id
    }

    pub fn item_bonuses(&self) -> &'a PlayerItemBonusStateLikeCpp {
        self.item_bonuses
    }

    pub fn aura_view(&self) -> PlayerStatAuraView<'a> {
        self.aura_view
    }

    pub fn previous_weapon_damage_pct(&self) -> [f32; 3] {
        self.previous_weapon_damage_pct
    }

    pub fn main_hand(&self) -> Option<&PlayerStatWeaponFacts> {
        self.main_hand.as_ref()
    }

    pub fn off_hand(&self) -> Option<&PlayerStatWeaponFacts> {
        self.off_hand.as_ref()
    }

    pub fn ranged(&self) -> Option<&PlayerStatWeaponFacts> {
        self.ranged.as_ref()
    }
}

fn weapon_facts(
    inventory: &wow_entities::PlayerInventoryRuntime,
    slot: u8,
) -> Option<PlayerStatWeaponFacts> {
    let item = inventory.inventory_items().get(&slot)?;
    let object = inventory.item_objects().get(&item.guid)?;
    if object.is_broken() {
        return None;
    }

    // C++ Player::UpdateDamageDoneMods reads each weapon enchantment slot
    // (`Player.cpp:4991-5015`). Keep the current conversion to u32 (nonnegative
    // IDs, including zero) used by the Inventory planner and do not copy Item.
    let enchantment_ids = object
        .data()
        .enchantments
        .iter()
        .filter_map(|enchantment| u32::try_from(enchantment.id).ok())
        .collect();

    Some(PlayerStatWeaponFacts {
        entry_id: item.entry_id,
        enchantment_ids,
    })
}
