//! Values updates and packets published for represented item state.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

pub(crate) fn item_push_result_from_send_new_item_plan(
    plan: &SendNewItemPlan,
) -> wow_packet::packets::item::ItemPushResult {
    wow_world_inventory::item_push_result_from_send_new_item_plan(plan)
}

impl WorldSession {
    pub(crate) fn inventory_equip_cx_like_cpp(&mut self) -> wow_world_application::InventoryEquipCxLikeCpp<'_> {
        #[cfg(any(test, feature = "test-fixtures"))]
        let stats_player = self.core.player_stats_access_with_fixture_refs_like_cpp(
            &self.catalogs,
            &self.config,
            &self.fixtures.identity.player_race,
            &self.fixtures.identity.player_class,
            &self.fixtures.identity.player_level,
            wow_world_core::session::StatsFixtureRefs::new_like_cpp(
                wow_world_core::session::StatsCombatFixtureRefs::new_like_cpp(
                    &mut self.fixtures.combat.player_health_like_cpp,
                    &mut self.fixtures.combat.player_max_health_like_cpp,
                    &mut self.fixtures.combat.player_alive_like_cpp,
                    &mut self.fixtures.combat.represented_player_powers_like_cpp[0],
                    &mut self.fixtures.combat.represented_player_max_powers_like_cpp[0],
                    &mut self.fixtures.combat.represented_player_base_mana_like_cpp,
                ),
                wow_world_core::session::StatsAuraFixtureRefs::new_like_cpp(
                    &self.fixtures.auras.represented_shapeshift_form_like_cpp,
                    &self.fixtures.auras.player_aura_authority_complete_like_cpp,
                    &self.fixtures.auras.player_spell_hit_aura_authority_tombstoned_like_cpp,
                    &self.fixtures.auras.visible_auras,
                    &self.fixtures.auras.canonical_threat_aura_snapshots_like_cpp,
                ),
            ),
        );
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let stats_player = self.core.player_stats_access_like_cpp(&self.catalogs, &self.config);
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
            &self.fixtures.progression.player_skill_test_fixture_like_cpp.player_skill_records_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))] &self.fixtures.identity.player_level,
            #[cfg(any(test, feature = "test-fixtures"))] &self.fixtures.progression.represented_primary_specialization_id_like_cpp,
        );
        #[cfg(any(test, feature = "test-fixtures"))]
        let hydration = if cfg!(test) {
            Some(wow_world_application::PlayerRegistryHydrationContext::new(
                self.core.player_registry_hydration_access_like_cpp(),
                &self.spell_state, &self.quest_state,
                (&self.fixtures.vehicles.player_mount_vehicle_kit_like_cpp,
                 &self.fixtures.vehicles.player_vehicle_seat_flags_like_cpp,
                 &self.fixtures.vehicles.player_vehicle_seat_id_like_cpp,
                 &self.fixtures.pets.represented_pet_guid_like_cpp),
                true,
            ))
        } else { None };
        wow_world_application::InventoryEquipCxLikeCpp::new(
            &mut self.inventory, &self.lifecycle,
            self.core.owned_inventory_access_like_cpp(), self.core.owned_item_modifiers_access_like_cpp(),
            item_sets, self.core.packet_publication_access_like_cpp(), stats_player,
            self.core.inventory_valuation_access_like_cpp(),
            self.catalogs.items.store.as_ref(), self.catalogs.items.stats_store.as_ref(),
            self.catalogs.items.effect_store.as_ref(),
            self.catalogs.spell_catalogs.spell_item_enchantment_store.as_deref(),
            wow_world_inventory::ItemModsCatalogsViewLikeCpp::new(
                self.catalogs.items.store.as_ref(), self.catalogs.items.stats_store.as_ref(),
                self.catalogs.scaling_stat_distribution_store.as_ref(), self.catalogs.scaling_stat_values_store.as_ref(),
                self.catalogs.shield_block_regular_game_table.as_ref(),
                self.catalogs.spell_catalogs.spell_shapeshift_form_store(),
            ),
            #[cfg(any(test, feature = "test-fixtures"))] self.catalogs.inventory_valuation_catalog_view_like_cpp(),
            &self.loot,
            self.core.player_registry_sync_access_like_cpp(
                #[cfg(any(test, feature = "test-fixtures"))] &self.fixtures.movement.player_position,
                #[cfg(any(test, feature = "test-fixtures"))] &self.fixtures.combat.player_health_like_cpp,
                #[cfg(any(test, feature = "test-fixtures"))] &self.fixtures.combat.player_max_health_like_cpp,
                #[cfg(any(test, feature = "test-fixtures"))] &self.fixtures.combat.player_alive_like_cpp,
                #[cfg(any(test, feature = "test-fixtures"))] &self.fixtures.identity.player_level,
                #[cfg(any(test, feature = "test-fixtures"))] &self.fixtures.vehicles.player_transport_login_state_like_cpp,
            ),
            #[cfg(any(test, feature = "test-fixtures"))]
            wow_world_application::InventoryEquipFixtureRefsLikeCpp::new(
                &self.fixtures.identity.player_level, &self.fixtures.auras.represented_shapeshift_form_like_cpp,
                &self.fixtures.auras.visible_auras,
            ),
            cfg!(test),
            #[cfg(any(test, feature = "test-fixtures"))] Self::MIN_ITEM_LEVEL_LIKE_CPP,
            #[cfg(any(test, feature = "test-fixtures"))] Self::MAX_ITEM_LEVEL_LIKE_CPP,
            #[cfg(any(test, feature = "test-fixtures"))] hydration,
        )
    }
    pub(crate) fn inventory_position_publication_cx_like_cpp(&mut self) -> wow_world_application::InventoryPositionPublicationCxLikeCpp<'_> {
        #[cfg(any(test, feature = "test-fixtures"))]
        let stats_player = self.core.player_stats_access_with_fixture_refs_like_cpp(
            &self.catalogs,
            &self.config,
            &self.fixtures.identity.player_race,
            &self.fixtures.identity.player_class,
            &self.fixtures.identity.player_level,
            wow_world_core::session::StatsFixtureRefs::new_like_cpp(
                wow_world_core::session::StatsCombatFixtureRefs::new_like_cpp(
                    &mut self.fixtures.combat.player_health_like_cpp,
                    &mut self.fixtures.combat.player_max_health_like_cpp,
                    &mut self.fixtures.combat.player_alive_like_cpp,
                    &mut self.fixtures.combat.represented_player_powers_like_cpp[0],
                    &mut self.fixtures.combat.represented_player_max_powers_like_cpp[0],
                    &mut self.fixtures.combat.represented_player_base_mana_like_cpp,
                ),
                wow_world_core::session::StatsAuraFixtureRefs::new_like_cpp(
                    &self.fixtures.auras.represented_shapeshift_form_like_cpp,
                    &self.fixtures.auras.player_aura_authority_complete_like_cpp,
                    &self.fixtures.auras.player_spell_hit_aura_authority_tombstoned_like_cpp,
                    &self.fixtures.auras.visible_auras,
                    &self.fixtures.auras.canonical_threat_aura_snapshots_like_cpp,
                ),
            ),
        );
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let stats_player = self.core.player_stats_access_like_cpp(&self.catalogs, &self.config);
        wow_world_application::InventoryPositionPublicationCxLikeCpp::new(
            &self.inventory, self.core.owned_inventory_access_like_cpp(),
            self.core.packet_publication_access_like_cpp(),
            self.catalogs.items.store.as_ref(), self.catalogs.items.stats_store.as_ref(), stats_player,
        )
    }
    pub(in crate::session) fn broadcast_item_push_result_to_group(&self, bytes: Vec<u8>) -> bool {
        let (Some(group_guid), Some(group_registry), Some(player_registry)) = (
            self.resolved_group_guid_like_cpp(),
            &self.core.directory.group_registry,
            &self.core.player_registry,
        ) else {
            return false;
        };

        let Some(group) = group_registry.get(&group_guid) else {
            return false;
        };

        let mut delivered = false;
        for member_guid in &group.members {
            if let Some(member) = player_registry.loot_presence(*member_guid) {
                delivered |= player_registry
                    .send_current_realm_packet(member.registration, bytes.clone())
                    .is_ok();
            }
        }

        delivered
    }
    pub(crate) fn send_item_relocation_values_update_like_cpp(
        &self,
        item_guid: ObjectGuid,
        dynamic_flags2_changed: bool,
        cleared_enchantments: &[EnchantmentSlot],
    ) {
        self.inventory.send_item_relocation_values_update_with_access_like_cpp(
            &self.core.owned_inventory_access_like_cpp(),
            &self.core.packet_publication_access_like_cpp(),
            item_guid,
            dynamic_flags2_changed,
            cleared_enchantments,
        )
    }
    pub(crate) fn send_item_dynamic_flags_values_update_like_cpp(&self, item_guid: ObjectGuid) {
        self.inventory.send_item_dynamic_flags_values_update_with_access_like_cpp(
            &self.core.owned_inventory_access_like_cpp(),
            &self.core.packet_publication_access_like_cpp(), item_guid,
        )
    }
    pub(crate) fn send_repeatable_turn_in_request_items_like_cpp(
        &mut self,
        sender_guid: ObjectGuid,
        quest: &wow_data::quest::QuestTemplate,
    ) {
        let can_complete = self.can_complete_repeatable_quest_represented_bounded_like_cpp(quest);
        if can_complete && !quest_has_represented_item_objective_like_cpp(quest) {
            self.send_represented_quest_giver_offer_reward_like_cpp(sender_guid, quest, true);
            return;
        }

        self.send_represented_quest_giver_request_items_with_completion_like_cpp(
            sender_guid,
            quest,
            can_complete,
            true,
        );
    }
}
