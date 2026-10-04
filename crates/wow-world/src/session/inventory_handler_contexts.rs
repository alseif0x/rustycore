// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_world_inventory::{
    EquipmentSetsHandlerCxLikeCpp, EquipmentSetsSaveCxLikeCpp, InventoryHandlerHostLikeCpp,
};
use wow_core::EquipmentSetGuidGeneratorLikeCpp;

use super::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl wow_world_application::BankHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn bank_slot_flag_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> wow_world_application::BankSlotFlagApplicationCxLikeCpp<'a> {
        self.build_bank_slot_flag_handler_cx_like_cpp()
    }
}

impl WorldSession {
    pub(crate) fn build_item_text_query_handler_cx_like_cpp(&self)
        -> wow_world_inventory::ItemTextQueryHandlerCxLikeCpp<'_> {
        wow_world_inventory::ItemTextQueryHandlerCxLikeCpp::new(
            &self.inventory,
            self.core.owned_inventory_access_like_cpp(),
            self.core.packet_publication_access_like_cpp(),
        )
    }
    pub(crate) fn build_bank_slot_flag_handler_cx_like_cpp(&mut self)
        -> wow_world_application::BankSlotFlagApplicationCxLikeCpp<'_> {
        let npc = self.core.npc_interaction_access_with_selected_refs_like_cpp(
            self.catalogs.factions.store.as_deref(),
            self.catalogs.factions.template_store.as_deref(),
            self.catalogs.friendship_rep_reaction_store.as_deref(),
            #[cfg(any(test, feature = "test-fixtures"))]
            wow_world_core::session::NpcInteractionFixtureRefsLikeCpp::new(
                &self.fixtures.movement.player_position,
                &self.fixtures.identity.player_faction_template_like_cpp,
                &self.fixtures.identity.player_race,
                &self.fixtures.identity.player_class,
                &self.fixtures.combat.player_health_like_cpp,
                &self.fixtures.combat.player_max_health_like_cpp,
                &self.fixtures.combat.player_alive_like_cpp,
                &self.fixtures.progression.reputation_state_like_cpp,
                &self.fixtures.vehicles.taxi_destinations_like_cpp,
                &self.fixtures.vehicles.taxi_flight_state_like_cpp,
                &self.fixtures.vehicles.taxi_unit_flags_like_cpp,
                &self.fixtures.vehicles.taxi_mounted_like_cpp,
            ),
        );
        wow_world_application::BankSlotFlagApplicationCxLikeCpp::new(
            &mut self.inventory, &self.interaction, npc,
            self.core.owned_inventory_access_like_cpp(),
            self.core.packet_publication_access_like_cpp(),
            self.catalogs.items.store.as_ref(),
            self.catalogs.items.stats_store.as_ref(),
        )
    }

    pub(crate) fn bank_interaction_access_like_cpp(&self)
        -> wow_world_core::session::NpcInteractionAccessLikeCpp<'_> {
        self.core.npc_interaction_access_with_selected_refs_like_cpp(
            self.catalogs.factions.store.as_deref(),
            self.catalogs.factions.template_store.as_deref(),
            self.catalogs.friendship_rep_reaction_store.as_deref(),
            #[cfg(any(test, feature = "test-fixtures"))]
            wow_world_core::session::NpcInteractionFixtureRefsLikeCpp::new(
                &self.fixtures.movement.player_position,
                &self.fixtures.identity.player_faction_template_like_cpp,
                &self.fixtures.identity.player_race,
                &self.fixtures.identity.player_class,
                &self.fixtures.combat.player_health_like_cpp,
                &self.fixtures.combat.player_max_health_like_cpp,
                &self.fixtures.combat.player_alive_like_cpp,
                &self.fixtures.progression.reputation_state_like_cpp,
                &self.fixtures.vehicles.taxi_destinations_like_cpp,
                &self.fixtures.vehicles.taxi_flight_state_like_cpp,
                &self.fixtures.vehicles.taxi_unit_flags_like_cpp,
                &self.fixtures.vehicles.taxi_mounted_like_cpp,
            ),
        )
    }

    pub(crate) fn build_item_enchantment_handler_cx_like_cpp(&mut self)
        -> wow_world_inventory::ItemEnchantmentApplicationCxLikeCpp<'_> {
        let owner = self.core.owned_item_enchantment_access_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.progression.player_skill_test_fixture_like_cpp.player_skill_records_like_cpp,
        );
        wow_world_inventory::ItemEnchantmentApplicationCxLikeCpp::new(
            &mut self.inventory,
            self.core.owned_inventory_access_like_cpp(),
            owner,
            wow_world_inventory::ItemEnchantmentCatalogsLikeCpp::new(
                self.catalogs.spell_catalogs.spell_item_enchantment_store.as_deref(),
                self.catalogs.spell_catalogs.spell_item_enchantment_condition_store.as_deref(),
                self.catalogs.items.store.as_ref(),
                self.catalogs.items.stats_store.as_ref(),
                self.catalogs.gem_properties_store.as_deref(),
            ),
        )
    }

    pub(crate) fn build_equipment_sets_handler_cx_like_cpp<'a>(
        &'a mut self,
    ) -> EquipmentSetsHandlerCxLikeCpp<'a> {
        let owner = self.core.owned_equipment_sets_access_like_cpp();
        EquipmentSetsHandlerCxLikeCpp::new(&mut self.inventory, owner)
    }

    pub(crate) fn build_equipment_sets_save_handler_cx_like_cpp<'a>(
        &'a mut self,
        generator: &'a EquipmentSetGuidGeneratorLikeCpp,
    ) -> EquipmentSetsSaveCxLikeCpp<'a> {
        let equipment_sets = self.core.owned_equipment_sets_access_like_cpp();
        let inventory = self.core.owned_inventory_access_like_cpp();
        #[cfg(any(test, feature = "test-fixtures"))]
        let collections = self
            .core
            .owned_collections_access_like_cpp()
            .with_fixture_collections(&self.fixtures.collections);
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let collections = self.core.owned_collections_access_like_cpp();
        let item_modified_appearance_store = self
            .catalogs
            .item_modified_appearance_store()
            .map(|store| store.as_ref());
        let spell_item_enchantment_store = self
            .catalogs
            .spell_item_enchantment_store()
            .map(|store| store.as_ref());
        let publication = self.core.packet_publication_access_like_cpp();
        EquipmentSetsSaveCxLikeCpp::new(
            &mut self.inventory,
            equipment_sets,
            inventory,
            collections,
            item_modified_appearance_store,
            spell_item_enchantment_store,
            generator,
            publication,
        )
    }
}

impl InventoryHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn item_text_query_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> wow_world_inventory::ItemTextQueryHandlerCxLikeCpp<'a> {
        self.build_item_text_query_handler_cx_like_cpp()
    }

    fn item_enchantment_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> wow_world_inventory::ItemEnchantmentApplicationCxLikeCpp<'a> {
        self.build_item_enchantment_handler_cx_like_cpp()
    }

    fn equipment_sets_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> EquipmentSetsHandlerCxLikeCpp<'a> {
        self.build_equipment_sets_handler_cx_like_cpp()
    }

    fn equipment_sets_save_handler_cx_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> EquipmentSetsSaveCxLikeCpp<'a> {
        self.build_equipment_sets_save_handler_cx_like_cpp(
            catalogs.id_generators.equipment_set.as_ref(),
        )
    }
}
