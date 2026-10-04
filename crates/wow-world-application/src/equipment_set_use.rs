// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::sync::Arc;

use wow_constants::ClientOpcodes;
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry,
    PacketProcessing, RegistryBuilder, SessionStatus,
};
use crate::PlayerRegistrySyncContext;
#[cfg(any(test, feature = "test-fixtures"))]
use crate::PlayerRegistryHydrationContext;
use crate::stats::CharacterStatsApplicationCxLikeCpp;
use wow_core::ObjectGuid;
use wow_data::progression_rewards::{ScalingStatDistributionStore, ScalingStatValuesStore};
use wow_data::{
    ItemStatsStore, ItemStore, ShieldBlockRegularGameTableLikeCpp, SpellShapeshiftFormStore,
};
use wow_entities::{
    EQUIPMENT_SLOT_END, EQUIPMENT_SLOT_MAINHAND, EQUIPMENT_SLOT_OFFHAND,
    INVENTORY_SLOT_BAG_0,
};
use wow_packet::{ClientPacket, WorldPacket};
use wow_packet::packets::misc::{UseEquipmentSet, UseEquipmentSetResult};
use wow_world_core::session::{
    EquipmentSetCombatAccessLikeCpp, EquipmentSetUseAccessLikeCpp,
    OwnedInventoryAccessLikeCpp, OwnedItemModifiersAccessLikeCpp, OwnedItemSetAccessLikeCpp,
    PacketPublicationAccessLikeCpp, SessionCatalogs, SessionWorldConfig,
};
#[cfg(any(test, feature = "test-fixtures"))]
use wow_world_core::session::PlayerStatsAccessLikeCpp;
use wow_world_inventory::{
    InventoryState, represented_player_stat_changes_like_cpp,
};
use wow_world_loot::LootState;

/// Selected item-modifier stores retained without cloning their owning Arcs.
pub struct EquipmentSetUseItemModsStoresLikeCpp<'a> {
    item_store: Option<&'a Arc<ItemStore>>,
    item_stats_store: Option<&'a Arc<ItemStatsStore>>,
    scaling_stat_distribution_store: Option<&'a Arc<ScalingStatDistributionStore>>,
    scaling_stat_values_store: Option<&'a Arc<ScalingStatValuesStore>>,
    shield_block_regular_game_table: Option<&'a Arc<ShieldBlockRegularGameTableLikeCpp>>,
    spell_shapeshift_form_store: Option<&'a Arc<SpellShapeshiftFormStore>>,
}

impl<'a> EquipmentSetUseItemModsStoresLikeCpp<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        item_store: Option<&'a Arc<ItemStore>>,
        item_stats_store: Option<&'a Arc<ItemStatsStore>>,
        scaling_stat_distribution_store: Option<&'a Arc<ScalingStatDistributionStore>>,
        scaling_stat_values_store: Option<&'a Arc<ScalingStatValuesStore>>,
        shield_block_regular_game_table: Option<&'a Arc<ShieldBlockRegularGameTableLikeCpp>>,
        spell_shapeshift_form_store: Option<&'a Arc<SpellShapeshiftFormStore>>,
    ) -> Self {
        Self {
            item_store,
            item_stats_store,
            scaling_stat_distribution_store,
            scaling_stat_values_store,
            shield_block_regular_game_table,
            spell_shapeshift_form_store,
        }
    }

}

/// Only the Session fixture references read or written by this operation.
#[cfg(any(test, feature = "test-fixtures"))]
pub struct EquipmentSetUseFixtureRefsLikeCpp<'a> {
    player_position: &'a Option<wow_core::Position>,
    player_race: &'a u8,
    player_class: &'a u8,
    player_level: &'a u8,
    player_health: &'a mut u32,
    player_max_health: &'a mut u32,
    player_alive: &'a mut bool,
    player_powers_slot0: &'a mut Option<i32>,
    player_max_powers_slot0: &'a mut Option<i32>,
    player_base_mana: &'a mut i32,
    shapeshift_form: &'a u32,
    aura_authority_complete: &'a bool,
    spell_hit_aura_tombstoned: &'a bool,
    visible_auras: &'a std::collections::HashMap<u8, wow_entities::AuraApplicationLikeCpp>,
    canonical_threat_aura_snapshots:
        &'a std::collections::HashMap<u8, wow_entities::AuraThreatSnapshotLikeCpp>,
    player_transport:
        &'a Option<Box<wow_world_core::session::PlayerTransportLoginStateLikeCpp>>,
}

#[cfg(any(test, feature = "test-fixtures"))]
impl<'a> EquipmentSetUseFixtureRefsLikeCpp<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        player_position: &'a Option<wow_core::Position>,
        player_race: &'a u8,
        player_class: &'a u8,
        player_level: &'a u8,
        player_health: &'a mut u32,
        player_max_health: &'a mut u32,
        player_alive: &'a mut bool,
        player_powers_slot0: &'a mut Option<i32>,
        player_max_powers_slot0: &'a mut Option<i32>,
        player_base_mana: &'a mut i32,
        shapeshift_form: &'a u32,
        aura_authority_complete: &'a bool,
        spell_hit_aura_tombstoned: &'a bool,
        visible_auras: &'a std::collections::HashMap<u8, wow_entities::AuraApplicationLikeCpp>,
        canonical_threat_aura_snapshots:
            &'a std::collections::HashMap<u8, wow_entities::AuraThreatSnapshotLikeCpp>,
        player_transport:
            &'a Option<Box<wow_world_core::session::PlayerTransportLoginStateLikeCpp>>,
    ) -> Self {
        Self {
            player_position,
            player_race,
            player_class,
            player_level,
            player_health,
            player_max_health,
            player_alive,
            player_powers_slot0,
            player_max_powers_slot0,
            player_base_mana,
            shapeshift_form,
            aura_authority_complete,
            spell_hit_aura_tombstoned,
            visible_auras,
            canonical_threat_aura_snapshots,
            player_transport,
        }
    }

    fn stats_access_like_cpp<'b>(
        &'b mut self,
        owner: &'b EquipmentSetUseAccessLikeCpp<'_>,
        catalogs: &'b SessionCatalogs,
        config: &'b SessionWorldConfig,
    ) -> PlayerStatsAccessLikeCpp<'b> {
        owner.player_stats_access_with_fixture_refs_like_cpp(
            catalogs,
            config,
            self.player_race,
            self.player_class,
            self.player_level,
            wow_world_core::session::StatsFixtureRefs::new_like_cpp(
                wow_world_core::session::StatsCombatFixtureRefs::new_like_cpp(
                    &mut *self.player_health,
                    &mut *self.player_max_health,
                    &mut *self.player_alive,
                    &mut *self.player_powers_slot0,
                    &mut *self.player_max_powers_slot0,
                    &mut *self.player_base_mana,
                ),
                wow_world_core::session::StatsAuraFixtureRefs::new_like_cpp(
                    self.shapeshift_form,
                    self.aura_authority_complete,
                    self.spell_hit_aura_tombstoned,
                    self.visible_auras,
                    self.canonical_threat_aura_snapshots,
                ),
            ),
        )
    }
}

pub struct EquipmentSetUseContextLikeCpp<'a> {
    inventory: &'a mut InventoryState,
    owner: EquipmentSetUseAccessLikeCpp<'a>,
    inventory_access: OwnedInventoryAccessLikeCpp<'a>,
    modifier_access: OwnedItemModifiersAccessLikeCpp<'a>,
    item_sets: OwnedItemSetAccessLikeCpp<'a>,
    combat: EquipmentSetCombatAccessLikeCpp<'a>,
    item_mods: EquipmentSetUseItemModsStoresLikeCpp<'a>,
    catalogs: &'a SessionCatalogs,
    config: &'a SessionWorldConfig,
    publication: PacketPublicationAccessLikeCpp<'a>,
    loot: &'a LootState,
    #[cfg(any(test, feature = "test-fixtures"))]
    fixtures: EquipmentSetUseFixtureRefsLikeCpp<'a>,
    #[cfg(any(test, feature = "test-fixtures"))]
    registry_hydration: Option<PlayerRegistryHydrationContext<'a>>,
    consumer_test: bool,
}

impl<'a> EquipmentSetUseContextLikeCpp<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        inventory: &'a mut InventoryState,
        owner: EquipmentSetUseAccessLikeCpp<'a>,
        inventory_access: OwnedInventoryAccessLikeCpp<'a>,
        modifier_access: OwnedItemModifiersAccessLikeCpp<'a>,
        item_sets: OwnedItemSetAccessLikeCpp<'a>,
        combat: EquipmentSetCombatAccessLikeCpp<'a>,
        item_mods: EquipmentSetUseItemModsStoresLikeCpp<'a>,
        catalogs: &'a SessionCatalogs,
        config: &'a SessionWorldConfig,
        publication: PacketPublicationAccessLikeCpp<'a>,
        loot: &'a LootState,
        #[cfg(any(test, feature = "test-fixtures"))] fixtures: EquipmentSetUseFixtureRefsLikeCpp<'a>,
        #[cfg(any(test, feature = "test-fixtures"))]
        registry_hydration: Option<PlayerRegistryHydrationContext<'a>>,
        consumer_test: bool,
    ) -> Self {
        Self {
            inventory,
            owner,
            inventory_access,
            modifier_access,
            item_sets,
            combat,
            item_mods,
            catalogs,
            config,
            publication,
            loot,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures,
            #[cfg(any(test, feature = "test-fixtures"))]
            registry_hydration,
            consumer_test,
        }
    }

    pub fn handle_use_equipment_set_like_cpp(&mut self, mut packet: wow_packet::WorldPacket) {
        let request = match UseEquipmentSet::read(&mut packet) {
            Ok(request) => request,
            Err(error) => {
                tracing::warn!("Bad UseEquipmentSet: {error}");
                return;
            }
        };

        let (changed_equipment, item_mods_changed) = self.apply_equipment_set_like_cpp(&request);
        if changed_equipment {
            self.sync_player_registry_like_cpp();
        }
        if item_mods_changed {
            #[cfg(any(test, feature = "test-fixtures"))]
            let (stats_access, stats_publication) = {
                let Self {
                    owner,
                    catalogs,
                    config,
                    fixtures,
                    ..
                } = self;
                let publication = owner.packet_publication_access_like_cpp();
                (
                    fixtures.stats_access_like_cpp(owner, catalogs, config),
                    publication,
                )
            };
            #[cfg(not(any(test, feature = "test-fixtures")))]
            let (stats_access, stats_publication) = (
                self.owner
                    .player_stats_access_like_cpp(self.catalogs, self.config),
                self.owner.packet_publication_access_like_cpp(),
            );
            let mut stats = CharacterStatsApplicationCxLikeCpp::new(
                stats_access,
                &*self.inventory,
                stats_publication,
            );
            let sent = stats.send_stat_update_like_cpp();
            drop(stats);
            if !sent {
                self.send_raw_item_bonus_update_like_cpp();
            }
        }
        self.publication.send_packet(&UseEquipmentSetResult {
            guid: request.guid,
            reason: 0,
        });
    }

    fn apply_equipment_set_like_cpp(
        &mut self,
        request: &UseEquipmentSet,
    ) -> (bool, bool) {
        let ignored_guid = wow_core::ObjectGuid::new(0x0C00_0400_0000_0000_i64, -1_i64);
        let mut changed_equipment = false;
        let mut item_mods_changed = false;

        for (slot_index, set_item) in request.items.iter().enumerate() {
            let dst = slot_index as u8;
            if set_item.item == ignored_guid {
                continue;
            }

            if self.combat.resolved_in_combat_like_cpp() != Some(false)
                && dst != EQUIPMENT_SLOT_MAINHAND
                && dst != EQUIPMENT_SLOT_OFFHAND
            {
                continue;
            }

            if let Some((src, _item)) = self.inventory
                .represented_direct_inventory_slot_by_guid_for_equipment_set_like_cpp(
                    &self.inventory_access,
                    self.item_mods.item_store,
                    self.item_mods.item_stats_store,
                    set_item.item,
                )
            {
                if src == dst {
                    continue;
                }
                if let Some(mods_changed) = self.move_item_like_cpp(src, dst) {
                    item_mods_changed |= mods_changed;
                    changed_equipment |= dst < EQUIPMENT_SLOT_END;
                    changed_equipment |= src < EQUIPMENT_SLOT_END;
                }
                continue;
            }

            let Some(_equipped_item) = self.inventory.equipment_set_inventory_item_by_pos_like_cpp(
                &self.inventory_access,
                self.item_mods.item_store,
                self.item_mods.item_stats_store,
                INVENTORY_SLOT_BAG_0,
                dst,
            ) else {
                continue;
            };
            let Some(backpack_slot) = self
                .inventory
                .find_free_backpack_slot_for_equipment_set_like_cpp(&self.inventory_access)
            else {
                continue;
            };
            if let Some(mods_changed) = self.move_item_like_cpp(dst, backpack_slot) {
                item_mods_changed |= mods_changed;
                changed_equipment = true;
            }
        }

        (changed_equipment, item_mods_changed)
    }

    fn move_item_like_cpp(&mut self, src: u8, dst: u8) -> Option<bool> {
        let item_mods = &self.item_mods;
        self.inventory
            .move_direct_inventory_item_with_item_mods_for_equipment_set_like_cpp(
                &self.inventory_access,
                &self.modifier_access,
                &self.item_sets,
                item_mods.item_store,
                item_mods.item_stats_store,
                item_mods.scaling_stat_distribution_store,
                item_mods.scaling_stat_values_store,
                item_mods.shield_block_regular_game_table,
                item_mods.spell_shapeshift_form_store,
                src,
                dst,
                #[cfg(any(test, feature = "test-fixtures"))]
                &self.fixtures.player_level,
                #[cfg(any(test, feature = "test-fixtures"))]
                self.fixtures.shapeshift_form,
                self.consumer_test,
            )
    }

    fn sync_player_registry_like_cpp(&mut self) {
        #[cfg(any(test, feature = "test-fixtures"))]
        let sync_access = self.owner.player_registry_sync_capabilities_like_cpp(
            self.fixtures.player_position,
            self.fixtures.player_level,
            self.fixtures.player_transport,
        );
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let sync_access = self.owner.player_registry_sync_capabilities_like_cpp();
        let Some((position, control)) = sync_access else {
            return;
        };
        let sync = PlayerRegistrySyncContext::new(
            position,
            control,
            self.loot,
            #[cfg(any(test, feature = "test-fixtures"))]
            wow_world_core::session::RegistrySyncInputs::new_like_cpp(
                self.fixtures.player_health,
                self.fixtures.player_max_health,
                self.fixtures.player_alive,
            ),
        );
        #[cfg(any(test, feature = "test-fixtures"))]
        let sync = if let Some(hydration) = self.registry_hydration.take() {
            sync.with_fixture_hydration(hydration)
        } else {
            sync
        };
        sync.sync();
    }

    fn send_raw_item_bonus_update_like_cpp(&self) {
        let Some(guid) = self.owner.player_guid_like_cpp() else {
            return;
        };
        let Some(bonuses) = self
            .inventory
            .represented_item_bonus_state_for_equipment_set_use_like_cpp(&self.modifier_access)
        else {
            return;
        };
        let update = wow_packet::packets::update::UpdateObject::player_stat_update(
            guid,
            self.owner.player_map_id_like_cpp(),
            represented_player_stat_changes_like_cpp(&bonuses),
        );
        self.publication.send_packet(&update);
    }
}

/// Supplies the operation-specific equipment-set context to its generic
/// Application-owned packet registrar.
pub trait EquipmentSetUseHandlerHostLikeCpp<C> {
    fn equipment_set_use_handler_cx_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
    ) -> EquipmentSetUseContextLikeCpp<'a>;
}

fn handle_use_equipment_set_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    packet: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: EquipmentSetUseHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .equipment_set_use_handler_cx_like_cpp(catalogs)
            .handle_use_equipment_set_like_cpp(packet);
    })
}

/// Register the existing UseEquipmentSet packet entry through its Application
/// owner, preserving its login gate and in-place processing classification.
pub fn register_equipment_set_use_handler_like_cpp<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: EquipmentSetUseHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::UseEquipmentSet,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_use_equipment_set",
        handler: handle_use_equipment_set_thunk::<S, C>,
    })?;
    Ok(())
}

#[cfg(test)]
#[path = "equipment_set_use/registration_tests.rs"]
mod registration_tests;
