// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::AuraRemovalCxLikeCpp;
use std::sync::Arc;

/// Selected catalog references; construction performs no owner query or Arc clone.
pub struct AuraApplicationCatalogsLikeCpp<'a> {
    mount_capabilities: Option<&'a wow_data::MountCapabilityStore>,
    spell_store: Option<&'a Arc<wow_data::SpellStore>>,
    spell_aura_options_store: Option<&'a wow_data::SpellAuraOptionsStore>,
    spell_misc_store: Option<&'a wow_data::SpellMiscStore>,
    spell_duration_store: Option<&'a wow_data::SpellDurationStore>,
    spell_proc_store: Option<&'a wow_data::SpellProcStoreLikeCpp>,
    classes_store: Option<&'a wow_data::character_progression::ChrClassesStore>,
    difficulty_store: Option<&'a wow_data::DifficultyStore>,
    item_store: Option<&'a Arc<wow_data::ItemStore>>,
    item_stats: Option<&'a Arc<wow_data::ItemStatsStore>>,
    item_effects: Option<&'a Arc<wow_data::ItemEffectStore>>,
    form_store: Option<&'a wow_data::SpellShapeshiftFormStore>,
    map_difficulty_id: u8,
    display_info_store: Option<&'a wow_data::CreatureDisplayInfoStore>,
    display_info_extra_store: Option<&'a wow_data::CreatureDisplayInfoExtraStore>,
    model_data_store: Option<&'a wow_data::CreatureModelDataStore>,
    races_store: Option<&'a wow_data::character_progression::ChrRacesStore>,
    shapeshift_display_links_absent: Option<bool>,
    creature_template_store: Option<&'a wow_data::CreatureTemplateLifecycleStoreLikeCpp>,
    creature_model_info_store: Option<&'a wow_data::CreatureModelInfoStoreLikeCpp>,
    map_store: Option<&'a wow_data::map::MapStore>,
    item_mod_catalogs: wow_world_inventory::ItemModsCatalogsViewLikeCpp<'a>,
}

impl<'a> AuraApplicationCatalogsLikeCpp<'a> {
    pub fn new(
        mount_capabilities: Option<&'a wow_data::MountCapabilityStore>,
        spell_store: Option<&'a Arc<wow_data::SpellStore>>,
        spell_aura_options_store: Option<&'a wow_data::SpellAuraOptionsStore>,
        spell_misc_store: Option<&'a wow_data::SpellMiscStore>,
        spell_duration_store: Option<&'a wow_data::SpellDurationStore>,
        spell_proc_store: Option<&'a wow_data::SpellProcStoreLikeCpp>,
        classes_store: Option<&'a wow_data::character_progression::ChrClassesStore>,
        difficulty_store: Option<&'a wow_data::DifficultyStore>,
        item_store: Option<&'a Arc<wow_data::ItemStore>>,
        item_stats: Option<&'a Arc<wow_data::ItemStatsStore>>,
        item_effects: Option<&'a Arc<wow_data::ItemEffectStore>>,
        form_store: Option<&'a wow_data::SpellShapeshiftFormStore>,
        map_difficulty_id: u8,
        display_info_store: Option<&'a wow_data::CreatureDisplayInfoStore>,
        display_info_extra_store: Option<&'a wow_data::CreatureDisplayInfoExtraStore>,
        model_data_store: Option<&'a wow_data::CreatureModelDataStore>,
        races_store: Option<&'a wow_data::character_progression::ChrRacesStore>,
        shapeshift_display_links_absent: Option<bool>,
        creature_template_store: Option<&'a wow_data::CreatureTemplateLifecycleStoreLikeCpp>,
        creature_model_info_store: Option<&'a wow_data::CreatureModelInfoStoreLikeCpp>,
        map_store: Option<&'a wow_data::map::MapStore>,
        item_mod_catalogs: wow_world_inventory::ItemModsCatalogsViewLikeCpp<'a>,
    ) -> Self {
        Self {
            mount_capabilities,
            spell_store,
            spell_aura_options_store,
            spell_misc_store,
            spell_duration_store,
            spell_proc_store,
            classes_store,
            difficulty_store,
            item_store,
            item_stats,
            item_effects,
            form_store,
            map_difficulty_id,
            display_info_store,
            display_info_extra_store,
            model_data_store,
            races_store,
            shapeshift_display_links_absent,
            creature_template_store,
            creature_model_info_store,
            map_store,
            item_mod_catalogs,
        }
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
        form: &'a mut u32,
        class: &'a u8,
        level: &'a u8,
        quests: &'a wow_world_quest::SessionQuestState,
        seat_flags: &'a Option<i32>,
        seat_id: &'a Option<u32>,
        transport: &'a Option<Box<wow_world_core::session::PlayerTransportLoginStateLikeCpp>>,
        vehicle_id: &'a mut u32,
        accessories: &'a mut Vec<wow_entities::VehicleAccessory>,
        seat_count: &'a mut u8,
        usable_seat_count: &'a mut u8,
        vehicle_remove_requests: &'a mut u32,
        pet_control_enable_requests: &'a mut u32,
        pet_resummon_requests: &'a mut u32,
        collision_update_requests: &'a mut u32,
    ) -> Self {
        Self {
            form,
            class,
            level,
            scaling: super::item_scaling::AuraScalingFixtureRefsLikeCpp {
                quests,
                vehicle_seat_flags: seat_flags,
                vehicle_seat_id: seat_id,
                transport,
            },
            mount: super::mount_control::MountRemovalEvidenceRefsLikeCpp {
                vehicle_id,
                accessories,
                seat_count,
                usable_seat_count,
                vehicle_remove_requests,
                pet_control_enable_requests,
                pet_resummon_requests,
                collision_update_requests,
            },
        }
    }
}

impl<'a> AuraRemovalCxLikeCpp<'a> {
    pub(super) fn is_current_race_sickness_spell(&mut self, spell_id: i32) -> Option<bool> {
        let races_store = self.races_store?;
        let race_id = {
            let player = self.stats.reborrow_like_cpp(
                &self.player,
                #[cfg(any(test, feature = "test-fixtures"))]
                &*self.shapeshift_form,
            );
            player.player_race_like_cpp()
        };
        races_store
            .get(u32::from(race_id))
            .map(|race| race.res_sickness_spell_id == spell_id)
    }

    pub(super) fn stat_application_uses_values(&mut self, required_sickness: bool) -> bool {
        if required_sickness {
            return true;
        }
        let player = self.stats.reborrow_like_cpp(
            &self.player,
            #[cfg(any(test, feature = "test-fixtures"))]
            &*self.shapeshift_form,
        );
        player.defer_stat_packet_for_current_player() == Some(true)
    }

    pub fn new(
        spell: &'a mut wow_world_spell::SessionSpellState,
        inventory: &'a mut wow_world_inventory::InventoryState,
        player: wow_world_core::session::PlayerAuraRemovalAccessLikeCpp<'a>,
        mount: wow_world_core::session::AuraMountControlAccessLikeCpp<'a>,
        stats: wow_world_core::session::AuraStatsAccessBuilderLikeCpp<'a>,
        item_sets: wow_world_core::session::OwnedItemSetAccessLikeCpp<'a>,
        catalogs: AuraApplicationCatalogsLikeCpp<'a>,
        loot: &'a wow_world_loot::LootState,
        consumer_test: bool,
        #[cfg(any(test, feature = "test-fixtures"))] fixtures: AuraApplicationFixtureRefsLikeCpp<
            'a,
        >,
    ) -> Self {
        Self {
            spell,
            inventory,
            player,
            mount,
            stats,
            item_sets,
            loot,
            consumer_test,
            mount_capabilities: catalogs.mount_capabilities,
            spell_store: catalogs.spell_store,
            spell_aura_options_store: catalogs.spell_aura_options_store,
            spell_misc_store: catalogs.spell_misc_store,
            spell_duration_store: catalogs.spell_duration_store,
            spell_proc_store: catalogs.spell_proc_store,
            classes_store: catalogs.classes_store,
            difficulty_store: catalogs.difficulty_store,
            item_store: catalogs.item_store,
            item_stats: catalogs.item_stats,
            item_effects: catalogs.item_effects,
            form_store: catalogs.form_store,
            map_difficulty_id: catalogs.map_difficulty_id,
            display_info_store: catalogs.display_info_store,
            display_info_extra_store: catalogs.display_info_extra_store,
            model_data_store: catalogs.model_data_store,
            races_store: catalogs.races_store,
            shapeshift_display_links_absent: catalogs.shapeshift_display_links_absent,
            creature_template_store: catalogs.creature_template_store,
            creature_model_info_store: catalogs.creature_model_info_store,
            map_store: catalogs.map_store,
            item_mod_catalogs: catalogs.item_mod_catalogs,
            #[cfg(any(test, feature = "test-fixtures"))]
            shapeshift_form: fixtures.form,
            #[cfg(any(test, feature = "test-fixtures"))]
            player_class: fixtures.class,
            #[cfg(any(test, feature = "test-fixtures"))]
            player_level: fixtures.level,
            #[cfg(any(test, feature = "test-fixtures"))]
            scaling_fixture: fixtures.scaling,
            #[cfg(any(test, feature = "test-fixtures"))]
            mount_evidence: fixtures.mount,
        }
    }
}

/// Builds the aura-application context from the hub and the domain states that
/// own its participants, so World and application handlers share one
/// construction path. `world_test_consumer` is the host's World-test flag
/// (World passes `cfg!(test)`).
pub fn player_aura_application_cx_like_cpp<'a>(
    hub: wow_world_core::session::HubMut<'a>,
    spell_state: &'a mut wow_world_spell::SessionSpellState,
    inventory: &'a mut wow_world_inventory::InventoryState,
    loot: &'a wow_world_loot::LootState,
    #[cfg(any(test, feature = "test-fixtures"))]
    quest_state: &'a wow_world_quest::SessionQuestState,
    world_test_consumer: bool,
) -> AuraRemovalCxLikeCpp<'a> {
    let wow_world_core::session::HubMut {
        core,
        catalogs,
        config,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixtures,
    } = hub;
    let core: &'a wow_world_core::session::SessionCore = core;

    let presentation = core.player_aura_removal_access_like_cpp(
        #[cfg(any(test, feature = "test-fixtures"))]
        wow_world_core::session::AuraRemovalFixtureRefsLikeCpp::new(
            &mut fixtures.auras.player_aura_authority_complete_like_cpp,
            &mut fixtures
                .auras
                .player_spell_hit_aura_authority_tombstoned_like_cpp,
            &mut fixtures.auras.visible_auras,
            &mut fixtures.auras.canonical_threat_aura_snapshots_like_cpp,
            &mut fixtures.vehicles.player_mount_display_id_like_cpp,
            &mut fixtures.vehicles.player_mounted_like_cpp,
            &mut fixtures.presentation.player_unit_flags_like_cpp,
            &fixtures.presentation.player_object_scale_like_cpp,
        ),
    );
    let control = core.aura_mount_control_access_like_cpp(
        catalogs.creatures.display_info_store.as_deref(),
        catalogs.creatures.model_data_store.as_deref(),
        #[cfg(any(test, feature = "test-fixtures"))]
        wow_world_core::session::AuraMountControlFixtureRefsLikeCpp::new(
            &mut fixtures.vehicles.player_mount_vehicle_kit_like_cpp,
            &mut fixtures.movement.movement_counter_like_cpp,
            &mut fixtures.movement.player_collision_height_like_cpp,
            &fixtures.movement.player_position,
            &mut fixtures.movement.player_movement_flags_like_cpp,
            &fixtures.movement.player_movement_time_like_cpp,
            &mut fixtures
                .movement
                .represented_can_swim_to_fly_transition_like_cpp,
            &fixtures.identity.player_scale_duration_like_cpp,
            &fixtures.identity.player_race,
            &fixtures.identity.player_gender,
            wow_world_core::session::AuraDismountPetFixtureRefsLikeCpp::new(
                &fixtures.pets.represented_pet_guid_like_cpp,
                &mut fixtures.pets.represented_pet_react_state_like_cpp,
                &mut fixtures.pets.represented_pet_command_state_like_cpp,
                &mut fixtures.pets.represented_pet_stable_like_cpp,
                &mut fixtures
                    .pets
                    .represented_character_pet_rows_empty_authority_complete_like_cpp,
                &mut fixtures
                    .pets
                    .represented_temporary_unsummoned_pet_number_like_cpp,
                &mut fixtures.pets.represented_old_pet_spell_like_cpp,
                &mut fixtures.pets.temporary_mount_pet_react_state_like_cpp,
            ),
            &mut fixtures.movement.movement_speed_rates_like_cpp,
            &mut fixtures.movement.forced_speed_changes_like_cpp,
            &fixtures.pets.represented_pet_guid_like_cpp,
            &mut fixtures.pets.represented_pet_movement_speed_rates_like_cpp,
            &mut fixtures.pets.represented_pet_speed_propagations_like_cpp,
            &fixtures.combat.in_combat,
            &mut fixtures.movement.last_fall_time_like_cpp,
            &mut fixtures.movement.last_fall_z_like_cpp,
        ),
    );
    let stats = core.aura_stats_access_builder_like_cpp(
        catalogs,
        config,
        #[cfg(any(test, feature = "test-fixtures"))]
        wow_world_core::session::StatsCombatFixtureRefs::new_like_cpp(
            &mut fixtures.combat.player_health_like_cpp,
            &mut fixtures.combat.player_max_health_like_cpp,
            &mut fixtures.combat.player_alive_like_cpp,
            &mut fixtures.combat.represented_player_powers_like_cpp[0],
            &mut fixtures.combat.represented_player_max_powers_like_cpp[0],
            &mut fixtures.combat.represented_player_base_mana_like_cpp,
        ),
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures.identity.player_race,
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures.identity.player_class,
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures.identity.player_level,
    );
    let item_sets = core.owned_item_set_access_like_cpp(
        catalogs.items.set_store.as_deref(),
        catalogs.spell_catalogs.item_set_spell_store.as_deref(),
        catalogs.spell_catalogs.spell_store.as_deref(),
        catalogs.heirloom_store.as_deref(),
        catalogs.items.stats_store.as_deref(),
        catalogs.curve_store.as_deref(),
        catalogs.curve_point_store.as_deref(),
        catalogs.content_tuning_store.as_deref(),
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures
            .progression
            .player_skill_test_fixture_like_cpp
            .player_skill_records_like_cpp,
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures.identity.player_level,
        #[cfg(any(test, feature = "test-fixtures"))]
        &fixtures
            .progression
            .represented_primary_specialization_id_like_cpp,
    );
    let catalogs = AuraApplicationCatalogsLikeCpp::new(
        catalogs.mount_capability_store.as_deref(),
        catalogs.spell_catalogs.spell_store.as_ref(),
        catalogs.spell_catalogs.spell_aura_options_store.as_deref(),
        catalogs.spell_catalogs.spell_misc_store.as_deref(),
        catalogs.spell_catalogs.spell_duration_store.as_deref(),
        catalogs.spell_catalogs.spell_proc_store.as_deref(),
        catalogs.chr.classes_store.as_deref(),
        catalogs.difficulty_store().map(AsRef::as_ref),
        catalogs.items.store.as_ref(),
        catalogs.items.stats_store.as_ref(),
        catalogs.items.effect_store.as_ref(),
        catalogs
            .spell_catalogs
            .spell_shapeshift_form_store
            .as_deref(),
        core.current_map_difficulty_id_like_cpp(),
        catalogs.creatures.display_info_store.as_deref(),
        catalogs.creatures.display_info_extra_store.as_deref(),
        catalogs.creatures.model_data_store.as_deref(),
        catalogs.chr.races_store.as_deref(),
        catalogs.chr.shapeshift_display_links_absent,
        catalogs
            .creatures
            .template_lifecycle_store_like_cpp
            .as_deref(),
        catalogs.creatures.model_info_store.as_deref(),
        catalogs.map_store().map(AsRef::as_ref),
        wow_world_inventory::ItemModsCatalogsViewLikeCpp::new(
            catalogs.items.store.as_ref(),
            catalogs.items.stats_store.as_ref(),
            catalogs.scaling_stat_distribution_store.as_ref(),
            catalogs.scaling_stat_values_store.as_ref(),
            catalogs.shield_block_regular_game_table.as_ref(),
            catalogs.spell_catalogs.spell_shapeshift_form_store(),
        ),
    );
    AuraRemovalCxLikeCpp::new(
        spell_state,
        inventory,
        presentation,
        control,
        stats,
        item_sets,
        catalogs,
        loot,
        world_test_consumer,
        #[cfg(any(test, feature = "test-fixtures"))]
        AuraApplicationFixtureRefsLikeCpp::new(
            &mut fixtures.auras.represented_shapeshift_form_like_cpp,
            &fixtures.identity.player_class,
            &fixtures.identity.player_level,
            quest_state,
            &fixtures.vehicles.player_vehicle_seat_flags_like_cpp,
            &fixtures.vehicles.player_vehicle_seat_id_like_cpp,
            &fixtures.vehicles.player_transport_login_state_like_cpp,
            &mut fixtures.vehicles.player_mount_vehicle_id_like_cpp,
            &mut fixtures.vehicles.player_mount_vehicle_accessories_like_cpp,
            &mut fixtures.vehicles.player_mount_vehicle_seat_count_like_cpp,
            &mut fixtures
                .vehicles
                .player_mount_vehicle_usable_seat_count_like_cpp,
            &mut fixtures.vehicles.mount_vehicle_remove_requests_like_cpp,
            &mut fixtures.pets.mount_pet_control_enable_requests_like_cpp,
            &mut fixtures.pets.mount_pet_resummon_requests_like_cpp,
            &mut fixtures
                .vehicles
                .mount_collision_height_update_requests_like_cpp,
        ),
    )
}
