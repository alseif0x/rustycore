// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::AuraRemovalCxLikeCpp;
use std::sync::Arc;

/// Selected catalog references; construction performs no owner query or Arc clone.
pub struct AuraApplicationCatalogsLikeCpp<'a> {
    mount_capabilities: Option<&'a wow_data::MountCapabilityStore>,
    spell_store: Option<&'a Arc<wow_data::SpellStore>>,
    classes_store: Option<&'a wow_data::character_progression::ChrClassesStore>,
    difficulty_store: Option<&'a wow_data::DifficultyStore>,
    item_store: Option<&'a Arc<wow_data::ItemStore>>,
    item_stats: Option<&'a Arc<wow_data::ItemStatsStore>>,
    item_effects: Option<&'a Arc<wow_data::ItemEffectStore>>,
    form_store: Option<&'a wow_data::SpellShapeshiftFormStore>,
    map_store: Option<&'a wow_data::map::MapStore>,
    item_mod_catalogs: wow_world_inventory::ItemModsCatalogsViewLikeCpp<'a>,
}

impl<'a> AuraApplicationCatalogsLikeCpp<'a> {
    pub fn new(
        mount_capabilities: Option<&'a wow_data::MountCapabilityStore>,
        spell_store: Option<&'a Arc<wow_data::SpellStore>>,
        classes_store: Option<&'a wow_data::character_progression::ChrClassesStore>,
        difficulty_store: Option<&'a wow_data::DifficultyStore>,
        item_store: Option<&'a Arc<wow_data::ItemStore>>,
        item_stats: Option<&'a Arc<wow_data::ItemStatsStore>>,
        item_effects: Option<&'a Arc<wow_data::ItemEffectStore>>,
        form_store: Option<&'a wow_data::SpellShapeshiftFormStore>,
        map_store: Option<&'a wow_data::map::MapStore>,
        item_mod_catalogs: wow_world_inventory::ItemModsCatalogsViewLikeCpp<'a>,
    ) -> Self {
        Self { mount_capabilities, spell_store, classes_store, difficulty_store,
            item_store, item_stats, item_effects, form_store, map_store, item_mod_catalogs }
    }
}

#[cfg(any(test, feature = "test-fixtures"))]
pub struct AuraApplicationFixtureRefsLikeCpp<'a> {
    form: &'a mut u32,
    class: &'a u8,
    level: &'a u8,
    scaling: super::item_scaling::AuraScalingFixtureRefsLikeCpp<'a>,
    mount: super::mount_control::MountRemovalEvidenceRefsLikeCpp<'a>,
}

#[cfg(any(test, feature = "test-fixtures"))]
impl<'a> AuraApplicationFixtureRefsLikeCpp<'a> {
    pub fn new(
        form: &'a mut u32, class: &'a u8, level: &'a u8,
        quests: &'a crate::SessionQuestState,
        seat_flags: &'a Option<i32>, seat_id: &'a Option<u32>,
        transport: &'a Option<Box<wow_world_core::session::PlayerTransportLoginStateLikeCpp>>,
        vehicle_id: &'a mut u32, accessories: &'a mut Vec<wow_entities::VehicleAccessory>,
        seat_count: &'a mut u8, usable_seat_count: &'a mut u8,
        vehicle_remove_requests: &'a mut u32, pet_control_enable_requests: &'a mut u32,
        pet_resummon_requests: &'a mut u32, collision_update_requests: &'a mut u32,
    ) -> Self {
        Self {
            form, class, level,
            scaling: super::item_scaling::AuraScalingFixtureRefsLikeCpp {
                quests, vehicle_seat_flags: seat_flags, vehicle_seat_id: seat_id, transport,
            },
            mount: super::mount_control::MountRemovalEvidenceRefsLikeCpp {
                vehicle_id, accessories, seat_count, usable_seat_count, vehicle_remove_requests,
                pet_control_enable_requests, pet_resummon_requests, collision_update_requests,
            },
        }
    }
}

impl<'a> AuraRemovalCxLikeCpp<'a> {
    pub fn new(
        spell: &'a mut wow_world_spell::SessionSpellState,
        inventory: &'a mut wow_world_inventory::InventoryState,
        player: wow_world_core::session::PlayerAuraRemovalAccessLikeCpp<'a>,
        mount: wow_world_core::session::AuraMountControlAccessLikeCpp<'a>,
        stats: wow_world_core::session::AuraStatsAccessBuilderLikeCpp<'a>,
        item_sets: wow_world_core::session::OwnedItemSetAccessLikeCpp<'a>,
        catalogs: AuraApplicationCatalogsLikeCpp<'a>, loot: &'a wow_world_loot::LootState,
        consumer_test: bool,
        #[cfg(any(test, feature = "test-fixtures"))] fixtures: AuraApplicationFixtureRefsLikeCpp<'a>,
    ) -> Self {
        Self {
            spell, inventory, player, mount, stats, item_sets, loot, consumer_test,
            mount_capabilities: catalogs.mount_capabilities, spell_store: catalogs.spell_store,
            classes_store: catalogs.classes_store, difficulty_store: catalogs.difficulty_store,
            item_store: catalogs.item_store, item_stats: catalogs.item_stats,
            item_effects: catalogs.item_effects, form_store: catalogs.form_store,
            map_store: catalogs.map_store, item_mod_catalogs: catalogs.item_mod_catalogs,
            #[cfg(any(test, feature = "test-fixtures"))] shapeshift_form: fixtures.form,
            #[cfg(any(test, feature = "test-fixtures"))] player_class: fixtures.class,
            #[cfg(any(test, feature = "test-fixtures"))] player_level: fixtures.level,
            #[cfg(any(test, feature = "test-fixtures"))] scaling_fixture: fixtures.scaling,
            #[cfg(any(test, feature = "test-fixtures"))] mount_evidence: fixtures.mount,
        }
    }
}
