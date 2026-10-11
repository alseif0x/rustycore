use wow_constants::{SheathState, UnitPvpFlags, UnitStandStateType, WeaponAttackType};
use wow_core::ObjectGuid;
use wow_entities::{AccessorObjectKind, CreatureAddonLifecycleRecordLikeCpp};
use wow_loot::{OwnedLootAuthority, OwnedLootAuthorityLifecycle};
use wow_world_core::map_manager::{WorldCreature, pending_respawn_from_creature_like_cpp};
use wow_world_core::session::{HubMut, SharedCanonicalMapManager, power_type_from_u8_like_cpp};

use crate::WorldEntitiesState;

const CURRENT_EXPANSION_LIKE_CPP: u8 = wow_data::CREATURE_CURRENT_EXPANSION_LIKE_CPP as u8;

/// One canonical admission decision, taken under the canonical map manager lock.
///
/// R1: the canonical map's creature owns the one allocatable loot authority for
/// its object incarnation; the session loot-view state may hold aliases to that
/// allocation (#1263 F6-8E deleted the legacy store), but must never allocate a competing
/// claimable pool. The authority, the canonical owner's health timeline snapshot
/// and the aura provenance therefore travel together, so the registration caller
/// adopts authority and health timeline from the same decision instead of a
/// second manager lookup.
#[derive(Debug, Clone)]
pub struct CanonicalCreatureInsertOutcomeLikeCpp {
    pub loot_authority: OwnedLootAuthority,
    pub aura_provenance: Vec<(u8, u32, wow_entities::AuraCastProvenanceLikeCpp)>,
    /// Canonical owner snapshot read under the same lock that selected
    /// `loot_authority`.
    pub health_owner: wow_entities::Unit,
}

/// One canonical admission attempt.
///
/// R1's invariant is that one object incarnation has one allocatable loot
/// authority. `Refused` is the only *decision* about an incarnation: the
/// canonical owner already exists and the candidate would be a second
/// independently claimable pool, so nothing may be published for it.
/// `NoCanonicalManager`: the session has no canonical store, so there is
/// nothing to admit into and nothing is published.
///
/// `Deferred` is the R7b-1 boundary. A canonical map manager *is* configured,
/// but no canonical incarnation could admit the candidate — most importantly
/// because the manager has no map instance for the key yet ("grid loading
/// racing registration"). Such a candidate has no admitted owner, so the
/// mutation root would refuse every owner mutation on it for its whole life;
/// publication is therefore deferred until a canonical instance admits it
/// rather than publishing a creature whose mutations must always fail.
enum CanonicalCreatureAdmissionLikeCpp {
    NoCanonicalManager,
    Deferred,
    Refused,
    Admitted(CanonicalCreatureInsertOutcomeLikeCpp),
}

/// R7b-1 (#1263 F6-8D3b-1/D3b-2).
///
/// A deferred registration is held **on the canonical side**: in the canonical
/// map's own respawn queue when the instance exists, otherwise by the canonical
/// manager until it creates that instance
/// ([`wow_map::MapManager::defer_creature_respawn_like_cpp`]). The admitted
/// executor's lifecycle phase drains that queue and admits the spawn
/// canonically. Returns `false` only when no canonical manager can be locked.
fn defer_registration_to_canonical_side_like_cpp(
    canonical: Option<&SharedCanonicalMapManager>,
    map_id: u16,
    creature: &wow_entities::Creature,
) -> bool {
    let Some(Ok(mut canonical)) = canonical.map(|canonical| canonical.lock()) else {
        return false;
    };
    canonical.defer_creature_respawn_like_cpp(
        wow_map::MapKey::new(u32::from(map_id), 0),
        pending_respawn_from_creature_like_cpp(creature, std::time::Instant::now(), map_id),
    );
    true
}

pub fn insert_canonical_creature_map_object_on_map_like_cpp(
    manager: &SharedCanonicalMapManager,
    map_id: u32,
    instance_id: u32,
    creature: wow_entities::Creature,
) -> Option<CanonicalCreatureInsertOutcomeLikeCpp> {
    match admit_canonical_creature_map_object_on_map_like_cpp(
        manager,
        map_id,
        instance_id,
        creature,
    ) {
        CanonicalCreatureAdmissionLikeCpp::Admitted(outcome) => Some(outcome),
        CanonicalCreatureAdmissionLikeCpp::NoCanonicalManager
        | CanonicalCreatureAdmissionLikeCpp::Deferred
        | CanonicalCreatureAdmissionLikeCpp::Refused => None,
    }
}

fn admit_canonical_creature_map_object_on_map_like_cpp(
    manager: &SharedCanonicalMapManager,
    map_id: u32,
    instance_id: u32,
    creature: wow_entities::Creature,
) -> CanonicalCreatureAdmissionLikeCpp {
    let Ok(mut manager) = manager.lock() else {
        return CanonicalCreatureAdmissionLikeCpp::Deferred;
    };
    admit_canonical_creature_map_object_on_locked_map_like_cpp(
        &mut manager,
        map_id,
        instance_id,
        creature,
    )
}

/// [`insert_canonical_creature_map_object_on_map_like_cpp`] on a canonical
/// manager the caller already holds (#1263 F6-8D3a-3: the admitted executor
/// admits a respawn under its one canonical guard).
pub fn insert_canonical_creature_map_object_on_locked_map_like_cpp(
    manager: &mut wow_map::MapManager,
    map_id: u32,
    instance_id: u32,
    creature: wow_entities::Creature,
) -> Option<CanonicalCreatureInsertOutcomeLikeCpp> {
    match admit_canonical_creature_map_object_on_locked_map_like_cpp(
        manager,
        map_id,
        instance_id,
        creature,
    ) {
        CanonicalCreatureAdmissionLikeCpp::Admitted(outcome) => Some(outcome),
        CanonicalCreatureAdmissionLikeCpp::NoCanonicalManager
        | CanonicalCreatureAdmissionLikeCpp::Deferred
        | CanonicalCreatureAdmissionLikeCpp::Refused => None,
    }
}

fn admit_canonical_creature_map_object_on_locked_map_like_cpp(
    manager: &mut wow_map::MapManager,
    map_id: u32,
    instance_id: u32,
    mut creature: wow_entities::Creature,
) -> CanonicalCreatureAdmissionLikeCpp {
    use CanonicalCreatureAdmissionLikeCpp::{Admitted, Deferred, Refused};

    let guid = creature.unit().world().object().guid();
    let Some(map) = manager.find_map_mut(map_id, instance_id) else {
        return Deferred;
    };
    if map.map().get_creature(guid).is_some() {
        let Some(current) = map.map_mut().get_typed_creature_mut(guid) else {
            return Deferred;
        };
        let current_authority = current.loot_authority_like_cpp().clone();
        let incoming_authority = creature.loot_authority_like_cpp();
        // R1: the existing canonical creature already owns this incarnation's
        // authority. A duplicate admission is accepted only when it carries that
        // same allocation or an unused pristine
        // candidate. Any other candidate would be a second independently
        // claimable pool, so it is refused here instead of being reconciled with
        // the canonical pool; the canonical authority is returned unchanged and
        // nothing on the canonical object is mutated.
        if !incoming_authority.shares_storage_like_cpp(&current_authority)
            && !incoming_authority.is_pristine_like_cpp()
        {
            return Refused;
        }
        let health_owner = current.unit().clone();
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
        return Admitted(CanonicalCreatureInsertOutcomeLikeCpp {
            loot_authority: current_authority,
            aura_provenance,
            health_owner,
        });
    }
    if creature.loot_authority_like_cpp().lifecycle_like_cpp()
        == OwnedLootAuthorityLifecycle::Detached
    {
        return Deferred;
    }

    if map
        .map_mut()
        .settle_creature_addon_aura_provenance_like_cpp(&mut creature)
        .is_err()
    {
        return Deferred;
    }
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
    let Ok(record) = wow_entities::MapObjectRecord::new_creature(creature) else {
        return Deferred;
    };
    if map.map_mut().insert_map_object_record(record).is_err() {
        return Deferred;
    }
    // Fresh admission installs this candidate as the incarnation's single
    // authority; read the installed object back so the caller receives exactly
    // that allocation and the health timeline the map now owns.
    let Some((loot_authority, health_owner)) = map.map().with_creature_like_cpp(guid, |current| {
        (
            current.loot_authority_like_cpp().clone(),
            current.unit().clone(),
        )
    }) else {
        return Deferred;
    };
    Admitted(CanonicalCreatureInsertOutcomeLikeCpp {
        loot_authority,
        aura_provenance,
        health_owner,
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
        // #1263 F6-8D3a-3: the canonical incarnation keeps the create-time
        // class/power projection its legacy bridge is built with, so a respawn
        // entry built from either carries the same CREATE fields.
        //
        // #1263 F6-8D3b-2: the incarnation is now the creature's only
        // representation, so it receives the runtime installation (motion
        // master, respawn create projection) and the default waypoint
        // initialization its legacy bridge used to receive.
        WorldCreature::install_bridge_runtime_like_cpp(&mut canonical_creature, &create_data);
        if canonical_creature.default_movement_type()
            == wow_entities::MovementGeneratorType::Waypoint
        {
            let resolver = hub.catalogs.waypoint_path_resolver_like_cpp.clone();
            wow_world_core::map_manager::CreatureMovementLikeCpp::canonical_like_cpp(
                &mut canonical_creature,
            )
            .initialize_default_waypoint_movement_with_path_resolver_like_cpp(
                |path_id| resolver.as_ref().and_then(|resolver| resolver(path_id)),
            );
        }
        // R1: admission selects the incarnation's canonical authority; a
        // refused admission means the canonical owner already exists and this
        // candidate would be a second claimable pool, so nothing is published.
        //
        // R7b-1: a configured canonical manager whose admission could not
        // install an incarnation (no map instance for this key, or an
        // installation failure) leaves the candidate with no admitted owner, so
        // its publication is deferred on the canonical side until a canonical
        // instance admits it (F6-8D3b-2). #1263 F6-8E: the canonical
        // incarnation is the creature's only representation; no other store
        // publishes a copy of it.
        let admission = self.admit_canonical_creature_map_object_like_cpp(
            hub,
            map_id,
            canonical_creature.clone(),
        );
        if !matches!(admission, CanonicalCreatureAdmissionLikeCpp::Deferred) {
            return;
        }
        defer_registration_to_canonical_side_like_cpp(
            hub.core.canonical_map_manager.as_ref(),
            map_id,
            &canonical_creature,
        );
    }

    fn admit_canonical_creature_map_object_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        map_id: u16,
        creature: wow_entities::Creature,
    ) -> CanonicalCreatureAdmissionLikeCpp {
        let Some(manager) = hub.core.canonical_map_manager.as_ref() else {
            return CanonicalCreatureAdmissionLikeCpp::NoCanonicalManager;
        };
        admit_canonical_creature_map_object_on_map_like_cpp(manager, u32::from(map_id), 0, creature)
    }

    /// Remove one represented creature from the session's current map and
    /// return it (its incarnation, with its create projection).
    ///
    /// The canonical incarnation is removed (C++ `RemoveFromMap`) and returned
    /// with its create projection (#1263 F6-8E: the only creature store).
    pub fn remove_world_creature(
        &mut self,
        hub: &mut HubMut<'_>,
        guid: ObjectGuid,
    ) -> Option<WorldCreature> {
        let (map_id, instance_id) = hub.core.current_runtime_map_key_like_cpp();
        let canonical = hub.core.canonical_map_manager.as_ref().and_then(|manager| {
            let mut manager = manager.lock().ok()?;
            let map = manager
                .find_map_mut(u32::from(map_id), instance_id)?
                .map_mut();
            let creature = map.get_typed_creature_mut(guid)?;
            creature.clear_loot_like_cpp();
            let creature = creature.clone();
            let _ = map.remove_from_map_like_cpp(guid, true);
            Some(creature)
        });
        canonical.map(|creature| {
            let create_data = WorldCreature::create_data_from_canonical_like_cpp(&creature);
            WorldCreature {
                creature,
                create_data,
            }
        })
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
