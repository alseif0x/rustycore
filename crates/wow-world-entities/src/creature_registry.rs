use wow_constants::{SheathState, UnitPvpFlags, UnitStandStateType, WeaponAttackType};
use wow_core::ObjectGuid;
use wow_entities::{AccessorObjectKind, CreatureAddonLifecycleRecordLikeCpp};
use wow_loot::{OwnedLootAuthority, OwnedLootAuthorityLifecycle};
use wow_world_core::map_manager::{world_to_grid_coords, WorldCreature};
use wow_world_core::session::{
    power_type_from_u8_like_cpp, reconcile_creature_loot_authority_mirrors_like_cpp,
    remove_canonical_creature_map_object_on_map_like_cpp, HubMut, SharedCanonicalMapManager,
};

use crate::WorldEntitiesState;

const CURRENT_EXPANSION_LIKE_CPP: u8 = wow_data::CREATURE_CURRENT_EXPANSION_LIKE_CPP as u8;

#[derive(Debug, Clone)]
pub struct CanonicalCreatureInsertOutcomeLikeCpp {
    pub loot_authority: OwnedLootAuthority,
    pub aura_provenance: Vec<(u8, u32, wow_entities::AuraCastProvenanceLikeCpp)>,
}

pub fn insert_canonical_creature_map_object_on_map_like_cpp(
    manager: &SharedCanonicalMapManager,
    map_id: u32,
    instance_id: u32,
    mut creature: wow_entities::Creature,
) -> Option<CanonicalCreatureInsertOutcomeLikeCpp> {
    let guid = creature.unit().world().object().guid();
    let Ok(mut manager) = manager.lock() else {
        return None;
    };
    let map = manager.find_map_mut(map_id, instance_id)?;
    if map.map().get_creature(guid).is_some() {
        let current = map.map_mut().get_typed_creature_mut(guid)?;
        let current_authority = current.loot_authority_like_cpp().clone();
        let incoming_authority = creature.loot_authority_like_cpp().clone();
        let current_stamp = current_authority.stamp_like_cpp();
        let incoming_stamp = incoming_authority.stamp_like_cpp();
        let authority = reconcile_creature_loot_authority_mirrors_like_cpp(
            &current_authority,
            current_stamp,
            &incoming_authority,
            incoming_stamp,
        );
        current.rebind_loot_authority_if_current_like_cpp(
            &current_authority,
            current_stamp,
            authority.clone(),
        )?;
        creature.adopt_loot_authority_for_snapshot_like_cpp(authority.clone());
        let aura_provenance = current
            .unit()
            .subsystems()
            .auras
            .visible_auras
            .iter()
            .filter_map(|(slot, aura)| {
                let provenance = current
                    .unit()
                    .subsystems()
                    .auras
                    .aura_cast_provenance_like_cpp(*slot);
                (!provenance.cast_id.is_empty()).then_some((*slot, aura.spell_id, provenance))
            })
            .collect();
        return Some(CanonicalCreatureInsertOutcomeLikeCpp {
            loot_authority: authority,
            aura_provenance,
        });
    }
    if creature.loot_authority_like_cpp().lifecycle_like_cpp()
        == OwnedLootAuthorityLifecycle::Detached
    {
        return None;
    }

    map.map_mut()
        .settle_creature_addon_aura_provenance_like_cpp(&mut creature)
        .ok()?;
    let aura_provenance = creature
        .unit()
        .subsystems()
        .auras
        .visible_auras
        .keys()
        .filter_map(|slot| {
            let aura = creature.unit().subsystems().auras.visible_auras.get(slot)?;
            let provenance = creature
                .unit()
                .subsystems()
                .auras
                .aura_cast_provenance_like_cpp(*slot);
            (!provenance.cast_id.is_empty()).then_some((*slot, aura.spell_id, provenance))
        })
        .collect();
    let object = creature.unit().world().clone();
    let _ = map
        .map_mut()
        .add_to_map_like_cpp(AccessorObjectKind::Creature, object);
    creature.unit_mut().world_mut().object_mut().add_to_world();
    let authority = creature.loot_authority_like_cpp().clone();
    let record = wow_entities::MapObjectRecord::new_creature(creature).ok()?;
    map.map_mut().insert_map_object_record(record).ok()?;
    Some(CanonicalCreatureInsertOutcomeLikeCpp {
        loot_authority: authority,
        aura_provenance,
    })
}

impl WorldEntitiesState {
    pub fn register_world_creature_with_flags_extra_movement_and_default_motion_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        map_id: u16,
        position: wow_core::Position,
        create_data: wow_packet::packets::update::CreatureCreateData,
        min_dmg: u32,
        max_dmg: u32,
        aggro_radius: f32,
        loot_id: u32,
        skin_loot_id: u32,
        gold_min: u32,
        gold_max: u32,
        respawn_delay_secs: u32,
        selected_equipment_id: u8,
        original_equipment_id: i8,
        script_name: String,
        string_id: Option<String>,
        addon: Option<CreatureAddonLifecycleRecordLikeCpp>,
        boss_id: Option<u32>,
        dungeon_encounter_id: u32,
        phase_use_flags: u8,
        phase_id: u16,
        phase_group_id: u32,
        terrain_swap_map: i32,
        flags_extra: u32,
        ground_movement_type: u8,
        swim_allowed: bool,
        flight_movement_type: u8,
        rooted: bool,
        chase_movement_type: u8,
        random_movement_type: u8,
        interaction_pause_timer_ms: u32,
        wander_distance: f32,
        default_movement_type: wow_entities::MovementGeneratorType,
        waypoint_path_id: u32,
    ) {
        let guid = create_data.guid;
        let entry = create_data.entry;
        let hp = create_data.health.max(1) as u32;
        let level = create_data.level;
        let display_id = create_data.display_id;
        let faction = create_data.faction_template.max(0) as u32;
        let npc_flags = create_data.npc_flags as u32;
        let npc_flags2 = (create_data.npc_flags >> 32) as u32;
        let unit_flags = create_data.unit_flags;
        let unit_flags2 = create_data.unit_flags2;
        let unit_flags3 = create_data.unit_flags3;
        let damage_school = create_data.damage_school;
        let (db_phase_shift, validated_terrain_swap_map) =
            hub.catalogs.db_spawn_phase_shift_like_cpp(
                map_id,
                phase_use_flags,
                phase_id,
                phase_group_id,
                terrain_swap_map,
            );
        let mut canonical_creature = {
            let mut creature = wow_entities::Creature::new(false);
            creature.unit_mut().world_mut().object_mut().create(guid);
            creature
                .unit_mut()
                .world_mut()
                .object_mut()
                .set_entry(entry);
            let _ = creature
                .unit_mut()
                .world_mut()
                .set_map(u32::from(map_id), 0);
            creature.unit_mut().world_mut().relocate(position);
            *creature.unit_mut().world_mut().phase_shift_mut() = db_phase_shift.clone();
            creature.unit_mut().set_level(level);
            creature.unit_mut().set_max_health(u64::from(hp));
            creature.unit_mut().set_health(u64::from(hp));
            creature.set_ai_identity_runtime(display_id, faction, npc_flags, unit_flags);
            creature.set_npc_flags2_runtime_like_cpp(npc_flags2);
            creature.set_unit_flags2_runtime_like_cpp(unit_flags2);
            creature.set_unit_flags3_runtime_like_cpp(unit_flags3);
            creature.set_melee_damage_school_like_cpp(damage_school);
            creature
                .unit_mut()
                .set_native_display_id_like_cpp(create_data.native_display_id);
            creature.unit_mut().set_display_scales_like_cpp(
                create_data.display_scale,
                create_data.native_x_display_scale,
            );
            creature
                .unit_mut()
                .set_bounding_radius(create_data.bounding_radius);
            creature
                .unit_mut()
                .set_combat_reach(create_data.combat_reach);
            creature
                .unit_mut()
                .set_hover_height_like_cpp(create_data.hover_height);
            let power_type = power_type_from_u8_like_cpp(create_data.display_power);
            // This legacy SQL path also becomes a typed canonical Creature. Keep
            // its display-power index and create mana coherent with the CREATE
            // arrays so later canonical reads and Unit power mutations address
            // the same slot as C++ `Creature::UpdateLevelDependantStats`.
            creature.set_power_type(power_type);
            creature
                .unit_mut()
                .set_create_mana_like_cpp(create_data.base_mana);
            creature
                .unit_mut()
                .replace_create_power_arrays_like_cpp(create_data.power, create_data.max_power);
            creature.unit_mut().set_base_attack_time_like_cpp(
                WeaponAttackType::BaseAttack,
                create_data.base_attack_time,
            );
            creature.unit_mut().set_base_attack_time_like_cpp(
                WeaponAttackType::OffAttack,
                create_data.base_attack_time,
            );
            creature.unit_mut().set_base_attack_time_like_cpp(
                WeaponAttackType::RangedAttack,
                create_data.ranged_attack_time,
            );
            creature
                .unit_mut()
                .set_mount_display_id(create_data.mount_display_id.max(0) as u32);
            creature
                .unit_mut()
                .set_stand_state_like_cpp(unit_stand_state_from_u8_like_cpp(
                    create_data.stand_state,
                ));
            creature
                .unit_mut()
                .replace_all_vis_flags_like_cpp(create_data.vis_flags);
            creature
                .unit_mut()
                .set_anim_tier_like_cpp(create_data.anim_tier);
            creature
                .unit_mut()
                .set_sheath_like_cpp(sheath_state_from_u8_like_cpp(create_data.sheathe_state));
            creature
                .unit_mut()
                .replace_all_pvp_flags_like_cpp(UnitPvpFlags::from_bits_retain(
                    create_data.pvp_flags,
                ));
            creature.set_flags_extra_runtime_like_cpp(flags_extra);
            creature.set_ground_movement_type_runtime_like_cpp(ground_movement_type);
            creature.set_swim_allowed_runtime_like_cpp(swim_allowed);
            creature.set_flight_movement_type_runtime_like_cpp(flight_movement_type);
            creature.set_template_rooted_like_cpp(rooted);
            creature.set_chase_movement_type_runtime_like_cpp(chase_movement_type);
            creature.set_random_movement_type_runtime_like_cpp(random_movement_type);
            creature.set_interaction_pause_timer_ms_runtime_like_cpp(interaction_pause_timer_ms);
            // This compatibility path has no DB CreatureTemplate. Real loaded-grid creatures carry
            // template RequiredExpansion through lifecycle metadata; legacy ad-hoc registrations
            // preserve the previous WotLK max-level behavior.
            creature.set_required_expansion_runtime_like_cpp(CURRENT_EXPANSION_LIKE_CPP);
            creature.set_default_movement_type_runtime_like_cpp(default_movement_type);
            creature.set_equipment_id_like_cpp(selected_equipment_id);
            creature.set_original_equipment_id_like_cpp(original_equipment_id);
            creature.set_ai_identity_names_runtime_like_cpp(String::new(), script_name);
            creature.set_spawn_string_id_runtime_like_cpp(string_id);
            creature.set_respawn_delay(respawn_delay_secs);
            if waypoint_path_id != 0 {
                creature.load_path_like_cpp(waypoint_path_id);
            }
            creature.apply_creatures_addon_lifecycle_like_cpp(addon.as_ref());
            let effective_waypoint_path_id = creature.waypoint_path_id_like_cpp();
            if effective_waypoint_path_id != 0 {
                creature.load_path_like_cpp(effective_waypoint_path_id);
            }
            creature.configure_ai_runtime(position, aggro_radius, wander_distance.max(0.0), 30);
            creature.ai_ownership_mut().respawn_time_secs = u64::from(respawn_delay_secs);
            creature.ai_ownership_mut().min_damage = min_dmg;
            creature.ai_ownership_mut().max_damage = max_dmg;
            creature.ai_ownership_mut().loot_id = loot_id;
            creature.ai_ownership_mut().skin_loot_id = skin_loot_id;
            creature.ai_ownership_mut().gold_min = gold_min;
            creature.ai_ownership_mut().gold_max = gold_max;
            creature.ai_ownership_mut().boss_id = boss_id;
            creature.ai_ownership_mut().dungeon_encounter_id = dungeon_encounter_id;
            creature.ai_ownership_mut().phase_use_flags = phase_use_flags;
            creature.ai_ownership_mut().phase_id = phase_id;
            creature.ai_ownership_mut().phase_group_id = phase_group_id;
            creature.ai_ownership_mut().terrain_swap_map = validated_terrain_swap_map;
            creature
        };
        canonical_creature.clear_data_changes();
        if let Some(outcome) = self.insert_canonical_creature_map_object_like_cpp(
            hub,
            map_id,
            canonical_creature.clone(),
        ) {
            canonical_creature.rebind_loot_authority_like_cpp(outcome.loot_authority);
            let _ = canonical_creature.take_pending_addon_aura_provenance_like_cpp();
            for (slot, spell_id, provenance) in outcome.aura_provenance {
                let auras = &mut canonical_creature.unit_mut().subsystems_mut().auras;
                if auras
                    .visible_auras
                    .get(&slot)
                    .is_some_and(|aura| aura.spell_id == spell_id)
                {
                    auras.set_aura_cast_provenance_like_cpp(slot, provenance);
                }
            }
        }
        let canonical_health_owner = hub.core.canonical_map_manager.as_ref().and_then(|manager| {
            let manager = manager.lock().ok()?;
            manager
                .find_map(u32::from(map_id), 0)?
                .map()
                .with_creature_like_cpp(guid, |current| current.unit().clone())
        });
        if let Some(current_unit) = canonical_health_owner {
            // When a canonical object pre-exists (for example grid loading
            // racing legacy registration), seed the compatibility mirror from
            // that exact health timeline rather than a separately constructed
            // Unit with incomparable revisions.
            canonical_creature
                .unit_mut()
                .preserve_authoritative_health_state_for_snapshot_like_cpp(&current_unit);
        }

        if let Some(manager) = &hub.core.map_manager {
            let (grid_x, grid_y) = world_to_grid_coords(position.x, position.y);
            let waypoint_path_resolver = hub.catalogs.waypoint_path_resolver_like_cpp.clone();
            let mut world_creature = WorldCreature::from_canonical(
                canonical_creature,
                create_data.clone(),
            );
            if world_creature.creature.default_movement_type()
                == wow_entities::MovementGeneratorType::Waypoint
            {
                world_creature.initialize_default_waypoint_movement_with_path_resolver_like_cpp(
                    |path_id| {
                        waypoint_path_resolver
                            .as_ref()
                            .and_then(|resolver| resolver(path_id))
                    },
                );
            }
            let mut manager = manager
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if manager.find_creature(map_id, 0, guid).is_none() {
                manager.add_creature(map_id, 0, grid_x, grid_y, world_creature);
            }
        }
    }

    fn insert_canonical_creature_map_object_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        map_id: u16,
        creature: wow_entities::Creature,
    ) -> Option<CanonicalCreatureInsertOutcomeLikeCpp> {
        let Some(manager) = hub.core.canonical_map_manager.as_ref() else {
            return None;
        };
        insert_canonical_creature_map_object_on_map_like_cpp(
            manager,
            u32::from(map_id),
            0,
            creature,
        )
    }

    pub fn remove_world_creature(
        &mut self,
        hub: &mut HubMut<'_>,
        guid: ObjectGuid,
    ) -> Option<WorldCreature> {
        let (map_id, instance_id) = hub.core.current_legacy_runtime_map_key_like_cpp();
        let manager = hub.core.map_manager.as_ref().cloned()?;
        let removed = {
            let mut manager = manager
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(creature) = manager.find_creature_mut(map_id, instance_id, guid) {
                creature.creature.clear_loot_like_cpp();
            }
            manager.remove_creature_any(map_id, instance_id, guid)
        };
        if removed.is_some() {
            self.remove_canonical_creature_map_object_like_cpp(hub, guid);
        }
        removed
    }

    fn remove_canonical_creature_map_object_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        guid: ObjectGuid,
    ) {
        let (map_id, instance_id) = hub.core.current_legacy_runtime_map_key_like_cpp();
        let Some(manager) = hub.core.canonical_map_manager.as_ref() else {
            return;
        };
        remove_canonical_creature_map_object_on_map_like_cpp(
            manager,
            u32::from(map_id),
            instance_id,
            guid,
        );
    }
}

pub const fn unit_stand_state_from_u8_like_cpp(value: u8) -> UnitStandStateType {
    match value {
        1 => UnitStandStateType::Sit,
        2 => UnitStandStateType::SitChair,
        3 => UnitStandStateType::Sleep,
        4 => UnitStandStateType::SitLowChair,
        5 => UnitStandStateType::SitMediumChair,
        6 => UnitStandStateType::SitHighChair,
        7 => UnitStandStateType::Dead,
        8 => UnitStandStateType::Kneel,
        9 => UnitStandStateType::Submerged,
        10 => UnitStandStateType::Max,
        _ => UnitStandStateType::Stand,
    }
}

pub const fn sheath_state_from_u8_like_cpp(value: u8) -> SheathState {
    match value {
        1 => SheathState::Melee,
        2 => SheathState::Ranged,
        _ => SheathState::Unarmed,
    }
}
