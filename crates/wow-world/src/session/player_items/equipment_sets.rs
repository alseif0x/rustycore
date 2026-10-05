//! Represented equipment sets and outfits.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

impl WorldSession {
    pub(crate) fn mark_represented_equipment_sets_loaded_like_cpp(&mut self) {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.mark_represented_equipment_sets_loaded_like_cpp(&mut hub)
    }
    pub(crate) fn load_represented_equipment_set_row_like_cpp(
        &mut self,
        guid: u64,
        set_id: u32,
        set_name: String,
        set_icon: String,
        ignore_mask: u32,
        assigned_spec_index: i32,
        pieces: [ObjectGuid; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
    ) -> bool {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.load_represented_equipment_set_row_like_cpp(
            &mut hub,
            guid,
            set_id,
            set_name,
            set_icon,
            ignore_mask,
            assigned_spec_index,
            pieces,
        )
    }
    pub(crate) fn represented_load_equipment_set_packet_like_cpp(
        &self,
    ) -> Option<wow_packet::packets::misc::LoadEquipmentSet> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.represented_load_equipment_set_packet_like_cpp(hub)
    }
    pub(crate) fn delete_represented_equipment_set_like_cpp(&mut self, id: u64) -> bool {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.delete_represented_equipment_set_like_cpp(&mut hub, id)
    }
    pub(crate) fn build_equipment_set_use_context_like_cpp(
        &mut self,
    ) -> wow_world_application::EquipmentSetUseContextLikeCpp<'_> {
        let combat = self.core.equipment_set_combat_access_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.combat.in_combat,
        );
        let owner = self.core.equipment_set_use_access_like_cpp();
        let inventory_access = self.core.owned_inventory_access_like_cpp();
        let modifier_access = self.core.owned_item_modifiers_access_like_cpp();
        let item_sets = self.core.owned_item_set_access_like_cpp(
            self.catalogs.items.set_store.as_deref(),
            self.catalogs.spell_catalogs.item_set_spell_store.as_deref(),
            self.catalogs.spell_catalogs.spell_store.as_deref(),
            self.catalogs.heirloom_store.as_deref(),
            self.catalogs.items.stats_store.as_deref(),
            self.catalogs.curve_store.as_deref(),
            self.catalogs.curve_point_store.as_deref(),
            self.catalogs.content_tuning_store.as_deref(),
            #[cfg(any(test, feature = "test-fixtures"))]
            &self
                .fixtures
                .progression
                .player_skill_test_fixture_like_cpp
                .player_skill_records_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.identity.player_level,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self
                .fixtures
                .progression
                .represented_primary_specialization_id_like_cpp,
        );
        let item_mods = wow_world_application::EquipmentSetUseItemModsStoresLikeCpp::new(
            self.catalogs.items.store.as_ref(),
            self.catalogs.items.stats_store.as_ref(),
            self.catalogs.scaling_stat_distribution_store.as_ref(),
            self.catalogs.scaling_stat_values_store.as_ref(),
            self.catalogs.shield_block_regular_game_table.as_ref(),
            self.catalogs.spell_catalogs.spell_shapeshift_form_store(),
        );
        #[cfg(test)]
        let registry_hydration = Some(wow_world_application::PlayerRegistryHydrationContext::new(
            self.core.player_registry_hydration_access_like_cpp(),
            &self.spell_state,
            &self.quest_state,
            (
                &self.fixtures.vehicles.player_mount_vehicle_kit_like_cpp,
                &self.fixtures.vehicles.player_vehicle_seat_flags_like_cpp,
                &self.fixtures.vehicles.player_vehicle_seat_id_like_cpp,
                &self.fixtures.pets.represented_pet_guid_like_cpp,
            ),
            true,
        ));
        #[cfg(all(not(test), feature = "test-fixtures"))]
        let registry_hydration = None;

        wow_world_application::EquipmentSetUseContextLikeCpp::new(
            &mut self.inventory,
            owner,
            inventory_access,
            modifier_access,
            item_sets,
            combat,
            item_mods,
            &self.catalogs,
            &self.config,
            self.core.packet_publication_access_like_cpp(),
            &self.loot,
            #[cfg(any(test, feature = "test-fixtures"))]
            wow_world_application::EquipmentSetUseFixtureRefsLikeCpp::new(
                &self.fixtures.movement.player_position,
                &self.fixtures.identity.player_race,
                &self.fixtures.identity.player_class,
                &self.fixtures.identity.player_level,
                &mut self.fixtures.combat.player_health_like_cpp,
                &mut self.fixtures.combat.player_max_health_like_cpp,
                &mut self.fixtures.combat.player_alive_like_cpp,
                &mut self.fixtures.combat.represented_player_powers_like_cpp[0],
                &mut self.fixtures.combat.represented_player_max_powers_like_cpp[0],
                &mut self.fixtures.combat.represented_player_base_mana_like_cpp,
                &self.fixtures.auras.represented_shapeshift_form_like_cpp,
                &self.fixtures.auras.player_aura_authority_complete_like_cpp,
                &self
                    .fixtures
                    .auras
                    .player_spell_hit_aura_authority_tombstoned_like_cpp,
                &self.fixtures.auras.visible_auras,
                &self.fixtures.auras.canonical_threat_aura_snapshots_like_cpp,
                &self.fixtures.vehicles.player_transport_login_state_like_cpp,
            ),
            #[cfg(any(test, feature = "test-fixtures"))]
            registry_hydration,
            cfg!(test),
        )
    }
    /// Set `ItemChildEquipment.db2`, used by C++ `CanEquipChildItem` and
    /// `EquipChildItem` to move a linked child into its visible equipment slot.
    pub fn set_item_child_equipment_store(&mut self, store: Arc<ItemChildEquipmentStore>) {
        self.catalogs.items.child_equipment_store = Some(store);
    }
    #[cfg(test)]
    pub fn set_creature_equipment_store_like_cpp(
        &mut self,
        store: Arc<CreatureEquipmentStoreLikeCpp>,
    ) {
        self.catalogs.creature_equipment_store_like_cpp = Some(store);
    }
    pub fn set_spell_equipped_items_store(&mut self, store: Arc<SpellEquippedItemsStore>) {
        self.catalogs.spell_catalogs.spell_equipped_items_store = Some(store);
    }
}

impl
    wow_world_application::EquipmentSetUseHandlerHostLikeCpp<
        crate::session::SessionHandlerCatalogsLikeCpp,
    > for WorldSession
{
    fn equipment_set_use_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a crate::session::SessionHandlerCatalogsLikeCpp,
    ) -> wow_world_application::EquipmentSetUseContextLikeCpp<'a> {
        self.build_equipment_set_use_context_like_cpp()
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/player_items/equipment_sets/f3_shims.rs"]
mod f3_shims;
