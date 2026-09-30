//! runtime access for the existing modifiers owner.

use super::*;

impl WorldSession {
    pub(in crate::session) fn player_item_modifier_runtime_snapshot_like_cpp(
        &self,
    ) -> Option<wow_entities::PlayerItemModifierRuntimeStateLikeCpp> {
        let canonical = self
            .with_owned_player_like_cpp(|player| player.item_modifier_runtime_snapshot_like_cpp());
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(
                self.player_item_test_fixture_like_cpp
                    .represented_item_modifier_runtime_like_cpp
                    .clone(),
            );
        }
        canonical
    }
    pub(in crate::session) fn apply_represented_item_bonus_action_state_like_cpp(
        &mut self,
        action: ApplyEnchantmentEffectAction,
    ) -> bool {
        let canonical = self.with_owned_player_mut_like_cpp(|player| {
            player.apply_item_modifier_action_like_cpp(action);
        });
        if canonical.is_some() {
            return true;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.ownerless_inventory_fallback_enabled_for_test() {
            self.player_item_test_fixture_like_cpp
                .represented_item_modifier_runtime_like_cpp
                .apply_enchantment_effect_action_like_cpp(action);
            return true;
        }
        false
    }
    pub(in crate::session) fn reset_represented_item_bonus_runtime_like_cpp(&mut self) {
        // C++ WorldSession::HandlePlayerLogin constructs a fresh Player, so
        // item modifiers from a previous character cannot survive into the
        // next login on the same session.
        #[cfg(test)]
        self.player_item_test_fixture_like_cpp
            .represented_item_bonus_actions_like_cpp
            .clear();
        let canonical_missing = self
            .with_owned_player_mut_like_cpp(|player| {
                player.reset_item_modifier_bonuses_like_cpp();
            })
            .is_none();
        #[cfg(not(test))]
        let _ = canonical_missing;
        #[cfg(test)]
        if canonical_missing && self.player_handle_like_cpp.is_none() {
            self.player_item_test_fixture_like_cpp
                .represented_item_modifier_runtime_like_cpp
                .reset_bonuses_like_cpp();
        }
    }

    /// C++ `Player::CanUseAttackType` for the attack an equipment slot maps to:
    /// `BASE_ATTACK` requires no `UNIT_FLAG_DISARMED`, `OFF_ATTACK` no
    /// `UNIT_FLAG2_DISARM_OFFHAND`, `RANGED_ATTACK` no
    /// `UNIT_FLAG2_DISARM_RANGED`, and any other slot is unaffected.
    ///
    /// `None` when the canonical Player owner is unavailable, so a caller can
    /// choose whether an unknown disarm state is fail-closed (enchantments) or
    /// fail-open (the `_ApplyWeaponDamage` producer, which C++ reaches for an
    /// unflagged unit).
    ///
    /// C++ `Player::GetAttackBySlot` has cases only for MAINHAND and OFFHAND.
    /// In particular, legacy `EQUIPMENT_SLOT_RANGED` deliberately falls through
    /// to `MAX_ATTACK`; ranged inventory types map to `RANGED_ATTACK` only when
    /// stored in MAINHAND.
    pub(in crate::session) fn represented_can_use_attack_type_like_cpp(
        &self,
        slot: u8,
        inventory_type: Option<InventoryType>,
    ) -> Option<bool> {
        let attack_type = match slot {
            EQUIPMENT_SLOT_MAINHAND
                if matches!(
                    inventory_type,
                    Some(InventoryType::Ranged | InventoryType::RangedRight)
                ) =>
            {
                WeaponAttackType::RangedAttack
            }
            EQUIPMENT_SLOT_MAINHAND => WeaponAttackType::BaseAttack,
            EQUIPMENT_SLOT_OFFHAND => WeaponAttackType::OffAttack,
            _ => WeaponAttackType::Max,
        };
        match attack_type {
            WeaponAttackType::BaseAttack => self.canonical_player_snapshot_like_cpp(|player| {
                !player
                    .unit()
                    .unit_flags_like_cpp()
                    .contains(UnitFlags::DISARMED)
            }),
            WeaponAttackType::OffAttack => self.canonical_player_snapshot_like_cpp(|player| {
                !player
                    .unit()
                    .unit_flags2_like_cpp()
                    .contains(UnitFlags2::DISARM_OFFHAND)
            }),
            WeaponAttackType::RangedAttack => self.canonical_player_snapshot_like_cpp(|player| {
                !player
                    .unit()
                    .unit_flags2_like_cpp()
                    .contains(UnitFlags2::DISARM_RANGED)
            }),
            WeaponAttackType::Max => Some(true),
        }
    }
    #[cfg(test)]
    pub(crate) fn represented_item_bonus_actions_like_cpp(
        &self,
    ) -> &[RepresentedItemBonusActionLikeCpp] {
        &self
            .player_item_test_fixture_like_cpp
            .represented_item_bonus_actions_like_cpp
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn represented_item_set_spell_events_like_cpp(
        &self,
    ) -> &[RepresentedItemSetSpellEventLikeCpp] {
        &self
            .player_item_test_fixture_like_cpp
            .represented_item_set_spell_events_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn represented_item_set_aura_refresh_events_like_cpp(
        &self,
    ) -> &[RepresentedItemSetAuraRefreshEventLikeCpp] {
        &self
            .player_item_test_fixture_like_cpp
            .represented_item_set_aura_refresh_events_like_cpp
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn represented_item_bonus_state_like_cpp(&self) -> RepresentedItemBonusStateLikeCpp {
        self.resolved_item_bonus_state_like_cpp()
            .expect("test Player item-bonus owner must resolve")
    }
    pub(crate) fn resolved_item_bonus_state_like_cpp(
        &self,
    ) -> Option<RepresentedItemBonusStateLikeCpp> {
        Some(
            self.player_item_modifier_runtime_snapshot_like_cpp()?
                .bonuses_snapshot_like_cpp(),
        )
    }

    /// C++ `Player::CanUseItem(ItemTemplate const*)`'s reputation term
    /// (`Player.cpp:11106-11107`): the player's `GetReputationRank` for the
    /// required faction. Returns `None` when the faction exists but the
    /// reputation manager is not represented, so each caller keeps its own
    /// fail-closed policy; an unknown faction ranks zero like C++.
    pub(crate) fn represented_item_reputation_rank_like_cpp(
        &self,
        required_reputation_faction: u32,
    ) -> Option<u32> {
        if required_reputation_faction == 0 {
            return Some(0);
        }
        let Some(faction) = self
            .factions
            .store
            .as_ref()
            .and_then(|store| store.get(required_reputation_faction))
        else {
            return Some(0);
        };
        // The session identity accessors re-enter the canonical manager lock
        // held by `with_reputation_mgr_like_cpp`, so resolve them first.
        let player_race = self.player_race_like_cpp();
        let player_class = self.player_class_like_cpp();
        let catalogs = crate::reputation_catalog_adapter::ReputationCatalogViewLikeCpp::new(
            None,
            self.friendship_rep_reaction_store.as_deref(),
            None,
            None,
        );
        let standing = self.with_reputation_mgr_like_cpp(|mgr| {
            mgr.reputation_for_faction_like_cpp(faction, player_race, player_class)
        })?;
        Some(u32::from(
            reputation_to_rank_like_cpp(
                faction,
                standing,
                &catalogs,
            )
            .as_u8(),
        ))
    }

    /// C++ `ItemTemplate::Effects` ordered by `ItemEffectEntry` slot. The
    /// `CanUseItem` learning-effect gate (`Player.cpp:11110-11113`) reads the
    /// first two entries.
    pub(crate) fn represented_item_effect_spell_ids_like_cpp(
        &self,
        item_id: u32,
    ) -> Vec<(u8, i32)> {
        let mut effects: Vec<(u8, i32)> = self
            .items
            .effect_store
            .as_ref()
            .map(|store| {
                store
                    .values()
                    .filter(|effect| effect.parent_item_id == item_id)
                    .map(|effect| (effect.legacy_slot_index, effect.spell_id))
                    .collect()
            })
            .unwrap_or_default();
        effects.sort_by_key(|(slot, _)| *slot);
        effects
    }
}
