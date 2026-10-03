// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

#[cfg(any(test, feature = "test-fixtures"))]
use crate::RepresentedItemModsReapplyEventLikeCpp;
use crate::{
    RepresentedItemBonusActionLikeCpp, RepresentedItemSetAuraRefreshEventLikeCpp,
    RepresentedItemSetSpellEventLikeCpp,
};
use wow_constants::{
    InventoryType, UnitFlags, UnitFlags2, WeaponAttackType,
};
use wow_core::ObjectGuid;
use wow_entities::{
    ApplyEnchantmentEffectAction, EQUIPMENT_SLOT_MAINHAND, EQUIPMENT_SLOT_OFFHAND,
    PlayerItemBonusStateLikeCpp, PlayerItemLevelCapsLikeCpp, PlayerItemSetEffectLikeCpp,
};
use wow_progression::reputation_to_rank_like_cpp;
use wow_world_core::session::{HubMut, HubRef};

pub fn represented_player_stat_changes_like_cpp(
    state: &wow_entities::PlayerItemBonusStateLikeCpp,
) -> wow_packet::packets::update::PlayerStatChanges {
    let mut changes = wow_packet::packets::update::PlayerStatChanges {
        base_mana: state.mana_base,
        base_health: state.health_base,
        attack_power: state.attack_power_total,
        ranged_attack_power: state.ranged_attack_power_total,
        stats: state.stats_base,
        stat_pos_buff: state.stats_base,
        armor: state.armor_base + state.armor_total + state.resistances_base[0],
        combat_ratings: state.combat_ratings,
        // This fixture has no aura/stat producers, so the item accumulator is
        // the whole represented `SpellBaseDamageBonusDone`/`HealingBonusDone`.
        mod_damage_done_pos: std::array::from_fn(|school| {
            if school == 0 {
                0
            } else {
                state.spell_power_bonus
            }
        }),
        mod_damage_done_neg: [0; 7],
        mod_healing_done_pos: state.spell_power_bonus,
        mod_damage_done_percent: [1.0; 7],
        shield_block: i32::try_from(state.shield_block_value).unwrap_or(i32::MAX),
        ..Default::default()
    };

    changes.min_damage =
        state.weapon_damage[wow_constants::WeaponAttackType::BaseAttack as usize][0];
    changes.max_damage =
        state.weapon_damage[wow_constants::WeaponAttackType::BaseAttack as usize][1];
    changes.min_ranged_damage =
        state.weapon_damage[wow_constants::WeaponAttackType::RangedAttack as usize][0];
    changes.max_ranged_damage =
        state.weapon_damage[wow_constants::WeaponAttackType::RangedAttack as usize][1];
    changes
}

impl crate::InventoryState {
    pub fn represented_heirloom_item_set_bonus_over_level_cap_like_cpp(
        &self,
        hub: HubRef<'_>,
        item_guid: ObjectGuid,
    ) -> bool {
        let Some(item_entry) = self
            .resolved_inventory_item_object_like_cpp(hub, item_guid)
            .map(|item| item.object().entry())
        else {
            return false;
        };
        if !hub
            .catalogs
            .heirloom_store
            .as_ref()
            .is_some_and(|store| store.get_by_item_id_like_cpp(item_entry).is_some())
        {
            return false;
        }

        let Some(template) = hub
            .catalogs
            .items
            .stats_store
            .as_ref()
            .and_then(|store| store.sparse_template(item_entry))
        else {
            return false;
        };
        let curve_id = template.player_level_to_item_level_curve_id_like_cpp();
        if curve_id == 0 {
            return false;
        }

        let Some((curve_store, curve_point_store)) = hub
            .catalogs
            .curve_store
            .as_ref()
            .zip(hub.catalogs.curve_point_store.as_ref())
        else {
            return false;
        };
        let Some((_min_level, max_level)) =
            curve_store.curve_x_axis_range_like_cpp(curve_point_store, curve_id)
        else {
            return false;
        };
        if !max_level.is_finite() || max_level < 0.0 {
            return false;
        }
        let mut max_level = max_level as u32;

        if let Some(content_tuning) = hub
            .catalogs
            .content_tuning_store
            .as_ref()
            .and_then(|store| {
                store.content_tuning_data_like_cpp(
                    template.scaling_stat_content_tuning_like_cpp(),
                    true,
                )
            })
        {
            max_level = max_level.min(u32::try_from(content_tuning.max_level).unwrap_or(0));
        }

        u32::from(hub.player_level_like_cpp()) > max_level
    }

    pub fn initial_loaded_item_mods_can_apply_like_cpp(
        &self,
        hub: HubRef<'_>,
        item_guid: ObjectGuid,
    ) -> bool {
        let Some(item) = self.resolved_inventory_item_object_like_cpp(hub, item_guid) else {
            return false;
        };
        // C++ `_ApplyAllItemMods` skips broken items before both
        // `ApplyItemEquipSpell` and `ApplyEnchantment`.
        if item.is_broken() {
            return false;
        }
        let inventory_type = hub
            .catalogs
            .item_storage_template(item.object().entry())
            .map(|template| template.inventory_type);
        self.represented_can_use_attack_type_like_cpp(hub, item.slot(), inventory_type)
            == Some(true)
    }

    pub fn player_item_modifier_runtime_snapshot_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<wow_entities::PlayerItemModifierRuntimeStateLikeCpp> {
        let access = hub.core.owned_item_modifiers_access_like_cpp();
        self.player_item_modifier_runtime_snapshot_with_access_like_cpp(&access)
    }

    fn player_item_modifier_runtime_snapshot_with_access_like_cpp(
        &self,
        access: &wow_world_core::session::OwnedItemModifiersAccessLikeCpp<'_>,
    ) -> Option<wow_entities::PlayerItemModifierRuntimeStateLikeCpp> {
        let canonical = access.item_modifier_runtime_snapshot_like_cpp();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && access.owner_handle_absent_like_cpp() {
            return Some(
                self.player_item_test_fixture_like_cpp
                    .represented_item_modifier_runtime_like_cpp
                    .clone(),
            );
        }
        canonical
    }

    pub fn add_player_item_set_item_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        item_set_id: u32,
        item_guid: ObjectGuid,
    ) -> Option<usize> {
        let access = hub.core.owned_item_modifiers_access_like_cpp();
        self.add_player_item_set_item_with_access_like_cpp(&access, item_set_id, item_guid)
    }

    fn add_player_item_set_item_with_access_like_cpp(
        &mut self,
        access: &wow_world_core::session::OwnedItemModifiersAccessLikeCpp<'_>,
        item_set_id: u32,
        item_guid: ObjectGuid,
    ) -> Option<usize> {
        let canonical = access.add_item_set_item_like_cpp(item_set_id, item_guid);
        if canonical.is_some() {
            return canonical;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if access.owner_handle_absent_like_cpp() {
            return Some(
                self.player_item_test_fixture_like_cpp
                    .represented_item_modifier_runtime_like_cpp
                    .add_item_set_item_like_cpp(item_set_id, item_guid),
            );
        }
        None
    }

    pub fn add_player_item_set_bonus_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        item_set_id: u32,
        spell_entry_id: u32,
    ) -> Option<bool> {
        let access = hub.core.owned_item_modifiers_access_like_cpp();
        self.add_player_item_set_bonus_with_access_like_cpp(&access, item_set_id, spell_entry_id)
    }

    fn add_player_item_set_bonus_with_access_like_cpp(
        &mut self,
        access: &wow_world_core::session::OwnedItemModifiersAccessLikeCpp<'_>,
        item_set_id: u32,
        spell_entry_id: u32,
    ) -> Option<bool> {
        let canonical = access.add_item_set_bonus_like_cpp(item_set_id, spell_entry_id);
        if canonical.is_some() {
            return canonical;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if access.owner_handle_absent_like_cpp() {
            return Some(
                self.player_item_test_fixture_like_cpp
                    .represented_item_modifier_runtime_like_cpp
                    .add_item_set_bonus_like_cpp(item_set_id, spell_entry_id),
            );
        }
        None
    }

    pub(crate) fn remove_player_item_set_item_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        item_set_id: u32,
        item_guid: ObjectGuid,
    ) -> Option<Option<usize>> {
        let access = hub.core.owned_item_modifiers_access_like_cpp();
        self.remove_player_item_set_item_with_access_like_cpp(&access, item_set_id, item_guid)
    }

    fn remove_player_item_set_item_with_access_like_cpp(
        &mut self,
        access: &wow_world_core::session::OwnedItemModifiersAccessLikeCpp<'_>,
        item_set_id: u32,
        item_guid: ObjectGuid,
    ) -> Option<Option<usize>> {
        let canonical = access.remove_item_set_item_like_cpp(item_set_id, item_guid);
        if canonical.is_some() {
            return canonical;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if access.owner_handle_absent_like_cpp() {
            return Some(
                self.player_item_test_fixture_like_cpp
                    .represented_item_modifier_runtime_like_cpp
                    .remove_item_set_item_like_cpp(item_set_id, item_guid),
            );
        }
        None
    }

    pub(crate) fn remove_player_item_set_bonus_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        item_set_id: u32,
        spell_entry_id: u32,
    ) -> Option<bool> {
        let access = hub.core.owned_item_modifiers_access_like_cpp();
        self.remove_player_item_set_bonus_with_access_like_cpp(&access, item_set_id, spell_entry_id)
    }

    fn remove_player_item_set_bonus_with_access_like_cpp(
        &mut self,
        access: &wow_world_core::session::OwnedItemModifiersAccessLikeCpp<'_>,
        item_set_id: u32,
        spell_entry_id: u32,
    ) -> Option<bool> {
        let canonical = access.remove_item_set_bonus_like_cpp(item_set_id, spell_entry_id);
        if canonical.is_some() {
            return canonical;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if access.owner_handle_absent_like_cpp() {
            return Some(
                self.player_item_test_fixture_like_cpp
                    .represented_item_modifier_runtime_like_cpp
                    .remove_item_set_bonus_like_cpp(item_set_id, spell_entry_id),
            );
        }
        None
    }

    pub(crate) fn drop_player_empty_item_set_effect_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        item_set_id: u32,
    ) -> Option<bool> {
        let access = hub.core.owned_item_modifiers_access_like_cpp();
        self.drop_player_empty_item_set_effect_with_access_like_cpp(&access, item_set_id)
    }

    fn drop_player_empty_item_set_effect_with_access_like_cpp(
        &mut self,
        access: &wow_world_core::session::OwnedItemModifiersAccessLikeCpp<'_>,
        item_set_id: u32,
    ) -> Option<bool> {
        let canonical = access.drop_empty_item_set_effect_like_cpp(item_set_id);
        if canonical.is_some() {
            return canonical;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if access.owner_handle_absent_like_cpp() {
            return Some(
                self.player_item_test_fixture_like_cpp
                    .represented_item_modifier_runtime_like_cpp
                    .drop_empty_item_set_effect_like_cpp(item_set_id),
            );
        }
        None
    }

    pub fn represented_item_bonus_player_stat_update_object_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<wow_packet::packets::update::UpdateObject> {
        let player_guid = hub.core.player_guid()?;
        let bonuses = self.resolved_item_bonus_state_like_cpp(hub)?;
        Some(
            wow_packet::packets::update::UpdateObject::player_stat_update(
                player_guid,
                hub.core.player_map_id_like_cpp(),
                represented_player_stat_changes_like_cpp(&bonuses),
            ),
        )
    }

    pub fn apply_represented_item_bonus_action_state_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        action: ApplyEnchantmentEffectAction,
    ) -> bool {
        let access = hub.core.owned_item_modifiers_access_like_cpp();
        self.apply_represented_item_bonus_action_with_access_like_cpp(&access, action)
    }

    fn apply_represented_item_bonus_action_with_access_like_cpp(
        &mut self,
        access: &wow_world_core::session::OwnedItemModifiersAccessLikeCpp<'_>,
        action: ApplyEnchantmentEffectAction,
    ) -> bool {
        let canonical = access.apply_item_modifier_action_like_cpp(action);
        if canonical.is_some() {
            return true;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if access.owner_handle_absent_like_cpp() {
            self.player_item_test_fixture_like_cpp
                .represented_item_modifier_runtime_like_cpp
                .apply_enchantment_effect_action_like_cpp(action);
            return true;
        }
        false
    }

    pub fn reset_represented_item_bonus_runtime_like_cpp(&mut self, hub: &mut HubMut<'_>) {
        let access = hub.core.owned_item_modifiers_access_like_cpp();
        self.reset_represented_item_bonus_runtime_with_access_like_cpp(&access);
    }

    fn reset_represented_item_bonus_runtime_with_access_like_cpp(
        &mut self,
        access: &wow_world_core::session::OwnedItemModifiersAccessLikeCpp<'_>,
    ) {
        // C++ WorldSession::HandlePlayerLogin constructs a fresh Player, so
        // item modifiers from a previous character cannot survive into the
        // next login on the same session.
        #[cfg(any(test, feature = "test-fixtures"))]
        self.player_item_test_fixture_like_cpp
            .represented_item_bonus_actions_like_cpp
            .clear();
        let canonical_missing = access.reset_item_modifier_bonuses_like_cpp().is_none();
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = canonical_missing;
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical_missing && access.owner_handle_absent_like_cpp() {
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
    pub fn represented_can_use_attack_type_like_cpp(
        &self,
        hub: HubRef<'_>,
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
            WeaponAttackType::BaseAttack => hub.core.canonical_player_snapshot_like_cpp(|player| {
                !player
                    .unit()
                    .unit_flags_like_cpp()
                    .contains(UnitFlags::DISARMED)
            }),
            WeaponAttackType::OffAttack => hub.core.canonical_player_snapshot_like_cpp(|player| {
                !player
                    .unit()
                    .unit_flags2_like_cpp()
                    .contains(UnitFlags2::DISARM_OFFHAND)
            }),
            WeaponAttackType::RangedAttack => {
                hub.core.canonical_player_snapshot_like_cpp(|player| {
                    !player
                        .unit()
                        .unit_flags2_like_cpp()
                        .contains(UnitFlags2::DISARM_RANGED)
                })
            }
            WeaponAttackType::Max => Some(true),
        }
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_item_bonus_actions_like_cpp(
        &self,
    ) -> &[RepresentedItemBonusActionLikeCpp] {
        &self
            .player_item_test_fixture_like_cpp
            .represented_item_bonus_actions_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_item_set_spell_events_like_cpp(
        &self,
    ) -> &[RepresentedItemSetSpellEventLikeCpp] {
        &self
            .player_item_test_fixture_like_cpp
            .represented_item_set_spell_events_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_item_set_aura_refresh_events_like_cpp(
        &self,
    ) -> &[RepresentedItemSetAuraRefreshEventLikeCpp] {
        &self
            .player_item_test_fixture_like_cpp
            .represented_item_set_aura_refresh_events_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_item_set_effect_like_cpp(
        &self,
        hub: HubRef<'_>,
        item_set_id: u32,
    ) -> Option<PlayerItemSetEffectLikeCpp> {
        self.player_item_modifier_runtime_snapshot_like_cpp(hub)?
            .item_set_effect_like_cpp(item_set_id)
            .cloned()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_item_bonus_state_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> PlayerItemBonusStateLikeCpp {
        self.resolved_item_bonus_state_like_cpp(hub)
            .expect("test Player item-bonus owner must resolve")
    }

    pub fn resolved_item_bonus_state_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<PlayerItemBonusStateLikeCpp> {
        Some(
            self.player_item_modifier_runtime_snapshot_like_cpp(hub)?
                .bonuses_snapshot_like_cpp(),
        )
    }

    pub fn represented_item_reputation_rank_like_cpp(
        &self,
        hub: HubRef<'_>,
        required_reputation_faction: u32,
    ) -> Option<u32> {
        if required_reputation_faction == 0 {
            return Some(0);
        }
        let Some(faction) = hub
            .catalogs
            .factions
            .store
            .as_ref()
            .and_then(|store| store.get(required_reputation_faction))
        else {
            return Some(0);
        };
        // The session identity accessors re-enter the canonical manager lock
        // held by `with_reputation_mgr_like_cpp`, so resolve them first.
        let player_race = hub.player_race_like_cpp();
        let player_class = hub.player_class_like_cpp();
        let standing = hub.with_reputation_mgr_like_cpp(|mgr| {
            mgr.reputation_for_faction_like_cpp(faction, player_race, player_class)
        })?;
        Some(u32::from(
            reputation_to_rank_like_cpp(
                faction,
                standing,
                hub.catalogs.friendship_rep_reaction_store.as_deref(),
            )
            .as_u8(),
        ))
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_player_item_level_caps_for_test_like_cpp(
        &mut self,
        caps: PlayerItemLevelCapsLikeCpp,
    ) {
        self.player_item_test_fixture_like_cpp
            .represented_item_modifier_runtime_like_cpp
            .set_item_level_caps_like_cpp(caps);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn record_represented_item_mod_reapply_event_for_test_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        slot: u8,
        apply: bool,
    ) {
        self.player_item_test_fixture_like_cpp
            .represented_item_mod_reapply_events_like_cpp
            .push(RepresentedItemModsReapplyEventLikeCpp {
                item_guid,
                slot,
                apply,
            });
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn record_represented_item_bonus_actions_for_test_like_cpp(
        &mut self,
        actions: &[RepresentedItemBonusActionLikeCpp],
    ) {
        self.player_item_test_fixture_like_cpp
            .represented_item_bonus_actions_like_cpp
            .extend(actions.iter().cloned());
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn record_represented_item_set_aura_refresh_events_for_test_like_cpp(
        &mut self,
        events: &[RepresentedItemSetAuraRefreshEventLikeCpp],
    ) {
        self.player_item_test_fixture_like_cpp
            .represented_item_set_aura_refresh_events_like_cpp
            .extend(events.iter().cloned());
    }
}
