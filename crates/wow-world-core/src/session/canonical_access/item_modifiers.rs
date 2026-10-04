// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_constants::{UnitFlags, UnitFlags2, WeaponAttackType};
use wow_core::ObjectGuid;
use wow_entities::{ApplyEnchantmentEffectAction, PlayerItemModifierRuntimeStateLikeCpp};

use crate::session::SessionCore;

/// Borrowed, typed access to canonical Player item-modifier state.
pub struct OwnedItemModifiersAccessLikeCpp<'a> {
    core: &'a SessionCore,
}

impl SessionCore {
    /// Build access that resolves only this session's generation-checked Player handle.
    pub fn owned_item_modifiers_access_like_cpp(&self) -> OwnedItemModifiersAccessLikeCpp<'_> {
        OwnedItemModifiersAccessLikeCpp { core: self }
    }
}

impl OwnedItemModifiersAccessLikeCpp<'_> {
    pub fn set_item_level_caps_like_cpp(
        &self,
        caps: wow_entities::PlayerItemLevelCapsLikeCpp,
    ) -> Option<()> {
        self.core.with_owned_player_mut_like_cpp(|player| {
            player.set_item_level_caps_like_cpp(caps);
        })
    }

    /// Snapshot item-modifier state through this session's generation-checked handle.
    /// The manager guard is released before the snapshot is returned.
    pub fn item_modifier_runtime_snapshot_like_cpp(
        &self,
    ) -> Option<PlayerItemModifierRuntimeStateLikeCpp> {
        self.core
            .with_owned_player_like_cpp(|player| player.item_modifier_runtime_snapshot_like_cpp())
    }

    /// Add an item-set item through the generation-checked handle; no GUID fallback is used.
    /// The manager guard is released before this method returns.
    pub fn add_item_set_item_like_cpp(
        &self,
        item_set_id: u32,
        item_guid: ObjectGuid,
    ) -> Option<usize> {
        self.core.with_owned_player_mut_like_cpp(|player| {
            player.add_item_set_item_like_cpp(item_set_id, item_guid)
        })
    }

    /// Add an item-set bonus through the generation-checked handle; no GUID fallback is used.
    /// The manager guard is released before this method returns.
    pub fn add_item_set_bonus_like_cpp(
        &self,
        item_set_id: u32,
        spell_entry_id: u32,
    ) -> Option<bool> {
        self.core.with_owned_player_mut_like_cpp(|player| {
            player.add_item_set_bonus_like_cpp(item_set_id, spell_entry_id)
        })
    }

    /// Remove an item-set item through the generation-checked handle; no GUID fallback is used.
    /// The nested option distinguishes a missing owner from an absent item-set item.
    /// The manager guard is released before this method returns.
    pub fn remove_item_set_item_like_cpp(
        &self,
        item_set_id: u32,
        item_guid: ObjectGuid,
    ) -> Option<Option<usize>> {
        self.core.with_owned_player_mut_like_cpp(|player| {
            player.remove_item_set_item_like_cpp(item_set_id, item_guid)
        })
    }

    /// Remove an item-set bonus through the generation-checked handle; no GUID fallback is used.
    /// The manager guard is released before this method returns.
    pub fn remove_item_set_bonus_like_cpp(
        &self,
        item_set_id: u32,
        spell_entry_id: u32,
    ) -> Option<bool> {
        self.core.with_owned_player_mut_like_cpp(|player| {
            player.remove_item_set_bonus_like_cpp(item_set_id, spell_entry_id)
        })
    }

    /// Drop an empty item-set effect through the generation-checked handle; no GUID fallback is
    /// used. The manager guard is released before this method returns.
    pub fn drop_empty_item_set_effect_like_cpp(&self, item_set_id: u32) -> Option<bool> {
        self.core.with_owned_player_mut_like_cpp(|player| {
            player.drop_empty_item_set_effect_like_cpp(item_set_id)
        })
    }

    /// Apply one item-modifier action through the generation-checked handle; no GUID fallback is
    /// used. The manager guard is released before this method returns.
    pub fn apply_item_modifier_action_like_cpp(
        &self,
        action: ApplyEnchantmentEffectAction,
    ) -> Option<()> {
        self.core.with_owned_player_mut_like_cpp(|player| {
            player.apply_item_modifier_action_like_cpp(action);
        })
    }

    /// Reset item-modifier bonuses through the generation-checked handle; no GUID fallback is
    /// used. The manager guard is released before this method returns.
    pub fn reset_item_modifier_bonuses_like_cpp(&self) -> Option<()> {
        self.core.with_owned_player_mut_like_cpp(|player| {
            player.reset_item_modifier_bonuses_like_cpp();
        })
    }

    /// Read a strict snapshot through this session's generation-checked Player handle.
    pub fn player_level_snapshot_like_cpp(&self) -> Option<u8> {
        self.core
            .with_owned_player_like_cpp(|player| player.level_like_cpp())
    }

    /// Apply the normal missing-owner fallback used by scaling queries.
    pub fn normal_player_level_fallback_like_cpp(&self) -> u8 {
        self.core.player_level_without_owned_player_like_cpp()
    }

    /// Read the canonical Unit form, falling back to the Player projection only when it is zero.
    pub fn shapeshift_form_snapshot_like_cpp(&self) -> Option<u32> {
        self.core.with_owned_player_like_cpp(|player| {
            let unit_form = u32::from(player.unit().shapeshift_form_id_like_cpp());
            if unit_form != 0 {
                unit_form
            } else {
                player.shapeshift_form_id_like_cpp()
            }
        })
    }

    /// Snapshot feral-form state through the canonical Player GUID lookup.
    pub fn is_in_feral_form_like_cpp(&self) -> Option<bool> {
        self.core
            .canonical_player_snapshot_like_cpp(|player| player.is_in_feral_form_like_cpp())
    }

    /// Snapshot the disarm flag for the requested attack type through the canonical Player GUID.
    pub fn can_use_weapon_attack_type_like_cpp(
        &self,
        attack_type: WeaponAttackType,
    ) -> Option<bool> {
        match attack_type {
            WeaponAttackType::BaseAttack => self.core.canonical_player_snapshot_like_cpp(|player| {
                !player
                    .unit()
                    .unit_flags_like_cpp()
                    .contains(UnitFlags::DISARMED)
            }),
            WeaponAttackType::OffAttack => self.core.canonical_player_snapshot_like_cpp(|player| {
                !player
                    .unit()
                    .unit_flags2_like_cpp()
                    .contains(UnitFlags2::DISARM_OFFHAND)
            }),
            WeaponAttackType::RangedAttack => {
                self.core.canonical_player_snapshot_like_cpp(|player| {
                    !player
                        .unit()
                        .unit_flags2_like_cpp()
                        .contains(UnitFlags2::DISARM_RANGED)
                })
            }
            WeaponAttackType::Max => Some(true),
        }
    }

    /// Whether a test-fixture session has no canonical Player handle installed.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn owner_handle_absent_like_cpp(&self) -> bool {
        self.core.player_handle_like_cpp.is_none()
    }
}
