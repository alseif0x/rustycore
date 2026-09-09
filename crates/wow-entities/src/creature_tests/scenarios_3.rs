//! Creature regressions, part 3 of 4.
//!
//! Moved out of the creature_tests.rs root under #648; every test is unchanged.

use super::*;

#[test]
fn creature_runtime_respawn_reloads_represented_addon_local_fields_like_cpp() {
    let mut record = creature_lifecycle_create_record();
    record.addon = Some(CreatureAddonLifecycleRecordLikeCpp {
        path_id: 9_002,
        mount_display_id: 22_222,
        stand_state: UnitStandStateType::Sit,
        vis_flags: 0x04,
        anim_tier: 3,
        sheath_state: SheathState::Melee,
        pvp_flags: UnitPvpFlags::SANCTUARY,
        emote: 0,
        ai_anim_kit_id: 44,
        movement_anim_kit_id: 55,
        melee_anim_kit_id: 66,
        visibility_distance_type: VisibilityDistanceTypeLikeCpp::Large,
        auras: vec![70_003],
        aura_applications: Vec::new(),
    });
    let mut creature = Creature::create_from_lifecycle(record);
    creature.unit_mut().set_mount_display_id(1);
    creature
        .unit_mut()
        .set_stand_state_like_cpp(UnitStandStateType::Sleep);
    creature.unit_mut().replace_all_vis_flags_like_cpp(0x02);
    creature.unit_mut().set_anim_tier_like_cpp(1);
    creature.unit_mut().set_sheath_like_cpp(SheathState::Ranged);
    creature.unit_mut().replace_all_pet_flags_like_cpp(0x03);
    creature
        .unit_mut()
        .set_shapeshift_form_like_cpp(ShapeShiftForm::CatForm);
    creature
        .unit_mut()
        .replace_all_pvp_flags_like_cpp(UnitPvpFlags::FFA_PVP);
    creature.unit_mut().set_emote_state_like_cpp(99);
    creature.unit_mut().set_ai_anim_kit_id_like_cpp(1);
    creature.unit_mut().set_movement_anim_kit_id_like_cpp(2);
    creature.unit_mut().set_melee_anim_kit_id_like_cpp(3);

    creature.set_death_state_runtime(DeathState::JustDied, 1_000);
    creature.set_death_state_runtime(DeathState::JustRespawned, 2_000);

    assert_eq!(
        creature.unit().data().mount_display_id,
        22_222,
        "C++ Creature::setDeathState(JUST_RESPAWNED) calls LoadCreaturesAddon after Unit::setDeathState(ALIVE)"
    );
    assert_eq!(
        creature.unit().stand_state_like_cpp(),
        UnitStandStateType::Sit
    );
    assert_eq!(creature.unit().vis_flags_like_cpp(), 0x04);
    assert_eq!(creature.unit().anim_tier_like_cpp(), 3);
    assert_eq!(creature.unit().sheath_like_cpp(), SheathState::Melee);
    assert_eq!(creature.unit().pet_flags_like_cpp(), 0);
    assert_eq!(
        creature.unit().shapeshift_form_like_cpp(),
        ShapeShiftForm::None
    );
    assert_eq!(
        creature.unit().pvp_flags_like_cpp(),
        UnitPvpFlags::SANCTUARY
    );
    assert_eq!(creature.unit().ai_anim_kit_id_like_cpp(), 44);
    assert_eq!(creature.unit().movement_anim_kit_id_like_cpp(), 55);
    assert_eq!(creature.unit().melee_anim_kit_id_like_cpp(), 66);
    assert_eq!(
        creature.waypoint_path_id_like_cpp(),
        9_002,
        "C++ respawn reload path calls LoadCreaturesAddon and preserves nonzero PathId"
    );
    assert_eq!(
        creature
            .unit()
            .world()
            .visibility_distance_override_like_cpp(),
        Some(VisibilityDistanceTypeLikeCpp::Large.distance_like_cpp())
    );
    assert!(
        creature
            .unit()
            .subsystems()
            .auras
            .has_aura_spell_like_cpp(70_003)
    );
    assert_eq!(
        creature.unit().emote_state_like_cpp(),
        0,
        "C++ addon emote 0 skips SetEmoteState; the preceding death path already cleared the emote"
    );
}

#[test]
fn creature_lifecycle_health_is_clamped_to_max_health() {
    let mut record = creature_lifecycle_create_record();
    record.stats.max_health = 100;
    record.stats.health = 150;

    let creature = Creature::create_from_lifecycle(record);

    assert_eq!(creature.unit().data().max_health, 100);
    assert_eq!(creature.unit().data().health, 100);
}

#[test]
fn creature_lifecycle_plan_preserves_trinity_critical_order() {
    let plan = CreatureLifecyclePlan::trinity_create_load_from_db();

    assert!(plan.occurs_before(
        CreatureLifecycleStep::LookupTemplateAndDifficulty,
        CreatureLifecycleStep::InitEntryAndCreateFromProto
    ));
    assert!(plan.occurs_before(
        CreatureLifecycleStep::RelocateAndValidatePosition,
        CreatureLifecycleStep::InitEntryAndCreateFromProto
    ));
    assert!(plan.occurs_before(
        CreatureLifecycleStep::SelectLevel,
        CreatureLifecycleStep::UpdateLevelDependantStats
    ));
    assert!(plan.occurs_before(
        CreatureLifecycleStep::UpdateLevelDependantStats,
        CreatureLifecycleStep::AddToMap
    ));
    assert_eq!(plan.steps().last(), Some(&CreatureLifecycleStep::AddToMap));
}

#[test]
fn creature_lifecycle_create_with_spawn_cleans_object_and_unit_masks() {
    let mut record = creature_lifecycle_create_record();
    record.spawn = Some(creature_lifecycle_spawn());

    let creature = Creature::create_from_lifecycle(record);

    assert_eq!(creature.unit().values_update().changed_object_type_mask, 0);
    assert_eq!(
        creature
            .unit()
            .world()
            .object()
            .values_update()
            .changed_object_type_mask,
        0
    );
    assert_eq!(creature.spawn_id(), 44_000);
}

#[test]
fn creature_runtime_just_died_sets_corpse_respawn_and_clears_combat_bridge_state() {
    let now = 10_000;
    let victim = ObjectGuid::new(1, 2);
    let player = ObjectGuid::new(1, 3);
    let melee_spell = CurrentSpellRef::new(400, Some(player), None);
    let generic_spell = CurrentSpellRef::new(401, Some(player), None).with_cast_time_ms(1_000);
    let channeled_spell = CurrentSpellRef::new(402, Some(player), None)
        .with_cast_time_ms(1_000)
        .with_state(SpellState::Delayed);
    let applied_death_removed = AppliedAuraRef::new(501, player, 0, 0x1);
    let applied_passive = AppliedAuraRef::new(502, player, 1, 0x1);
    let applied_death_persistent = AppliedAuraRef::new(503, player, 2, 0x1);
    let owned_death_removed = OwnedAuraRef::new(601, player, None);
    let owned_passive = OwnedAuraRef::new(602, player, None);
    let owned_death_persistent = OwnedAuraRef::new(603, player, None);
    let mut creature = Creature::new(false);
    creature.set_respawn_compatibility_mode(true);
    creature.set_respawn_delay(45);
    creature.set_corpse_delay(15, false);
    creature.unit_mut().set_max_health(200);
    creature.unit_mut().set_health(125);
    creature.set_power_type(PowerType::Energy);
    creature.unit_mut().set_max_power(PowerType::Energy, 100);
    creature.unit_mut().set_power(PowerType::Energy, 45);
    creature.unit_mut().set_emote_state_like_cpp(88);
    creature
        .unit_mut()
        .set_stand_state_like_cpp(UnitStandStateType::Sit);
    creature.unit_mut().add_unit_state(UnitState::MOVING.bits());
    creature
        .unit_mut()
        .subsystems_mut()
        .motion
        .start_spline(77, 1_000);
    creature
        .unit_mut()
        .subsystems_mut()
        .vehicle
        .enter_vehicle(ObjectGuid::new(1, 700), Some(1));
    creature
        .unit_mut()
        .subsystems_mut()
        .control
        .set_summon_slot(1, ObjectGuid::new(1, 701));
    creature
        .unit_mut()
        .subsystems_mut()
        .control
        .set_charmed(ObjectGuid::new(1, 702));
    creature
        .unit_mut()
        .subsystems_mut()
        .control
        .add_controlled(ObjectGuid::new(1, 703));
    creature
        .unit_mut()
        .set_unit_flags_like_cpp(UnitFlags::PET_IN_COMBAT);
    creature.unit_mut().set_npc_flags_like_cpp(0x40);
    creature.unit_mut().set_npc_flags2_like_cpp(0x2);
    creature.unit_mut().set_mount_display_id(1234);
    creature.set_movement_flags_runtime_like_cpp(
        MovementFlag::HOVER
            | MovementFlag::DISABLE_GRAVITY
            | MovementFlag::CAN_FLY
            | MovementFlag::FLYING,
    );
    creature
        .unit_mut()
        .subsystems_mut()
        .auras
        .modify_aura_state(AURA_STATE_DEFENSIVE, true);
    creature
        .unit_mut()
        .subsystems_mut()
        .auras
        .modify_aura_state(AURA_STATE_DEFENSIVE_2, true);
    {
        let auras = &mut creature.unit_mut().subsystems_mut().auras;
        for aura in [
            applied_death_removed,
            applied_passive,
            applied_death_persistent,
        ] {
            auras.add_applied(aura);
        }
        for aura in [owned_death_removed, owned_passive, owned_death_persistent] {
            auras.add_owned(aura);
        }
        auras.set_aura_death_policy_like_cpp(applied_passive.aura_ref(), true, false);
        auras.set_aura_death_policy_like_cpp(applied_death_persistent.aura_ref(), false, true);
        auras.set_aura_death_policy_like_cpp(owned_passive.aura_ref(), true, false);
        auras.set_aura_death_policy_like_cpp(owned_death_persistent.aura_ref(), false, true);
    }
    creature.unit_mut().subsystems_mut().auras.incr_diminishing(
        DIMINISHING_STUN,
        DiminishingLevel::Immune,
        1_000,
    );
    creature
        .unit_mut()
        .subsystems_mut()
        .spells
        .set_current_spell(CurrentSpellSlot::Melee, melee_spell);
    creature
        .unit_mut()
        .set_current_cast_spell(CurrentSpellSlot::Generic, generic_spell);
    creature
        .unit_mut()
        .subsystems_mut()
        .spells
        .set_current_spell(CurrentSpellSlot::Channeled, channeled_spell);
    creature.unit_mut().set_target(victim);
    creature.set_represented_spell_focus_like_cpp(9001, player, 1.25, true);
    creature.unit_mut().set_attacking(Some(victim));
    creature.unit_mut().world_mut().set_active(true);
    creature
        .unit_mut()
        .subsystems_mut()
        .combat
        .add_threat(player, 7.5);
    creature
        .unit_mut()
        .subsystems_mut()
        .combat
        .add_attacker(player);
    creature.set_tapped_by_player(player, &[]);

    let plan = creature.set_death_state_runtime(DeathState::JustDied, now);

    assert_eq!(creature.unit().death_state(), DeathState::Corpse);
    assert_eq!(creature.corpse_remove_time(), now + 15);
    assert_eq!(creature.respawn_time(), now + 45 + 15);
    assert_eq!(creature.unit().data().target, ObjectGuid::EMPTY);
    assert_eq!(
        creature.spell_focus_state_like_cpp().spell_id,
        None,
        "C++ Creature::setDeathState(JUST_DIED) releases spell focus before clearing target"
    );
    assert_eq!(
        creature.spell_focus_state_like_cpp().delay_ms,
        0,
        "C++ DoNotReacquireSpellFocusTarget cancels the delayed target snapback"
    );
    assert!(
        !creature.unit().has_unit_state(UnitState::FOCUSING.bits()),
        "C++ ReleaseSpellFocus clears UNIT_STATE_FOCUSING for AI_DOESNT_FACE_TARGET spells"
    );
    assert_eq!(creature.unit().attacking(), None);
    assert_eq!(creature.unit().data().health, 0);
    assert_eq!(creature.unit().get_power(PowerType::Energy), 0);
    assert_eq!(creature.unit().emote_state_like_cpp(), 0);
    assert_eq!(
        creature.unit().stand_state_like_cpp(),
        UnitStandStateType::Stand
    );
    assert!(
        !UnitState::from_bits_truncate(creature.unit().unit_state()).intersects(UnitState::MOVING),
        "C++ Unit::StopMoving clears UNIT_STATE_MOVING during StopOnDeath"
    );
    assert!(
        creature.unit().subsystems().motion.stopped,
        "C++ MotionMaster::StopOnDeath calls Unit::StopMoving"
    );
    assert!(
        creature.unit().subsystems().motion.spline.finalized,
        "C++ Unit::setDeathState(JUST_DIED) disables/interrupts the movement spline when StopOnDeath succeeds"
    );
    assert_eq!(
        creature.unit().npc_flags_like_cpp(),
        [0, 0],
        "C++ Creature::setDeathState(JUST_DIED) calls ReplaceAllNpcFlags(0) and ReplaceAllNpcFlags2(0)"
    );
    assert_eq!(
        creature.unit().data().mount_display_id,
        0,
        "C++ Creature::setDeathState(JUST_DIED) calls SetMountDisplayId(0)"
    );
    assert_eq!(
        creature.movement_flags_like_cpp(),
        MovementFlag::CAN_FLY | MovementFlag::FLYING,
        "C++ death calls SetHover(false,false) and SetDisableGravity(false,false), but does not unset CAN_FLY/FLYING here"
    );
    assert_eq!(
        creature.unit().current_spell(CurrentSpellSlot::Melee),
        Some(melee_spell)
    );
    assert_eq!(
        creature.unit().current_spell(CurrentSpellSlot::Generic),
        None
    );
    assert_eq!(
        creature.unit().current_spell(CurrentSpellSlot::Channeled),
        None
    );
    assert!(
        !creature
            .unit()
            .subsystems()
            .auras
            .has_applied(applied_death_removed),
        "C++ RemoveAllAurasOnDeath removes non-passive, non-death-persistent applied auras"
    );
    assert!(
        creature
            .unit()
            .subsystems()
            .auras
            .has_applied(applied_passive),
        "C++ RemoveAllAurasOnDeath preserves passive applied auras"
    );
    assert!(
        creature
            .unit()
            .subsystems()
            .auras
            .has_applied(applied_death_persistent),
        "C++ RemoveAllAurasOnDeath preserves death-persistent applied auras"
    );
    assert!(
        !creature
            .unit()
            .subsystems()
            .auras
            .has_owned(owned_death_removed),
        "C++ RemoveAllAurasOnDeath removes non-passive, non-death-persistent owned auras"
    );
    assert!(creature.unit().subsystems().auras.has_owned(owned_passive));
    assert!(
        creature
            .unit()
            .subsystems()
            .auras
            .has_owned(owned_death_persistent)
    );
    assert!(
        creature
            .unit()
            .subsystems()
            .auras
            .removed_auras
            .contains(&AuraRef::new(501, player))
    );
    assert!(
        creature
            .unit()
            .subsystems()
            .auras
            .removed_auras
            .contains(&AuraRef::new(601, player))
    );
    assert_eq!(
        creature.unit().subsystems().vehicle.vehicle_guid,
        None,
        "C++ Unit::setDeathState(non-alive) calls ExitVehicle before RemoveAllControlled"
    );
    assert!(
        creature
            .unit()
            .subsystems()
            .control
            .summon_slots
            .iter()
            .all(|guid| guid.is_empty()),
        "C++ Unit::setDeathState(non-alive) calls UnsummonAllTotems before RemoveAllControlled"
    );
    assert!(
        creature
            .unit()
            .subsystems()
            .control
            .controlled_guids
            .is_empty(),
        "C++ Unit::setDeathState(non-alive) calls RemoveAllControlled"
    );
    assert_eq!(creature.unit().subsystems().control.charmed_guid, None);
    assert!(
        !creature
            .unit()
            .unit_flags_like_cpp()
            .contains(UnitFlags::PET_IN_COMBAT),
        "C++ RemoveAllControlled clears UNIT_FLAG_PET_IN_COMBAT for non-pets"
    );
    assert!(
        !creature.unit().world().is_active(),
        "C++ Creature::setDeathState(JUST_DIED) calls setActive(false)"
    );
    assert!(
        !creature
            .unit()
            .subsystems()
            .auras
            .has_aura_state(AURA_STATE_DEFENSIVE)
    );
    assert!(
        !creature
            .unit()
            .subsystems()
            .auras
            .has_aura_state(AURA_STATE_DEFENSIVE_2)
    );
    assert_eq!(
        creature
            .unit()
            .subsystems()
            .auras
            .get_diminishing(DIMINISHING_STUN, 1_000),
        DiminishingLevel::Level1
    );
    assert_eq!(
        creature.unit().subsystems().combat.threat_value(player),
        None,
        "C++ Unit::setDeathState(JUST_DIED) calls CombatStop before death-state side effects"
    );
    assert!(creature.unit().subsystems().combat.attackers.is_empty());
    assert!(creature.runtime_state().save_respawn_requested);
    assert!(plan.contains(CreatureRuntimeAction::SaveRespawnTime));
    assert!(plan.contains(CreatureRuntimeAction::ReleaseSpellFocus));
    assert!(plan.contains(CreatureRuntimeAction::CancelSpellFocusReacquire));
    assert!(plan.contains(CreatureRuntimeAction::ClearTarget));
    assert!(
        !plan.contains(CreatureRuntimeAction::MoveFall),
        "the legacy death-state entry point has no map-height context, so it must not fake C++ MoveFall"
    );

    let mut non_compat = Creature::new(false);
    non_compat.set_respawn_compatibility_mode(false);
    non_compat.set_respawn_delay(45);
    non_compat.set_corpse_delay(15, false);
    non_compat.set_death_state_runtime(DeathState::JustDied, now);
    assert_eq!(non_compat.respawn_time(), now + 45);
    assert_eq!(non_compat.corpse_remove_time(), now + 15);
    assert_eq!(non_compat.unit().death_state(), DeathState::Corpse);
}

#[test]
fn creature_runtime_just_died_can_start_represented_move_fall_with_map_context_like_cpp() {
    let mut creature = Creature::new(false);
    creature.set_movement_flags_runtime_like_cpp(MovementFlag::HOVER);

    let plan = creature.set_death_state_runtime_with_fall_like_cpp(
        DeathState::JustDied,
        1_000,
        Some(CreatureDeathFallContextLikeCpp {
            is_underwater: false,
            has_valid_ground_height: true,
            vertical_delta: 12.5,
            movement_id: 77,
            duration_ms: 850,
        }),
    );

    assert!(
        plan.contains(CreatureRuntimeAction::MoveFall),
        "C++ Creature::setDeathState(JUST_DIED) calls MoveFall after clearing hover/gravity when the pre-clear state was flying/hovering"
    );
    assert_eq!(creature.unit().death_state(), DeathState::Corpse);
    assert_eq!(creature.movement_flags_like_cpp(), MovementFlag::empty());
    let current = creature
        .unit()
        .subsystems()
        .motion
        .current_movement_generator();
    assert_eq!(current.kind, MovementGeneratorKind::Effect);
    assert_eq!(current.movement_id, 77);
    assert_eq!(current.duration_ms, Some(850));
}

#[test]
fn creature_runtime_just_died_move_fall_honors_cpp_underwater_and_root_guards() {
    let mut underwater = Creature::new(false);
    underwater.set_movement_flags_runtime_like_cpp(MovementFlag::HOVER);
    let underwater_plan = underwater.set_death_state_runtime_with_fall_like_cpp(
        DeathState::JustDied,
        1_000,
        Some(CreatureDeathFallContextLikeCpp {
            is_underwater: true,
            has_valid_ground_height: true,
            vertical_delta: 12.5,
            movement_id: 77,
            duration_ms: 850,
        }),
    );
    assert!(!underwater_plan.contains(CreatureRuntimeAction::MoveFall));

    let mut rooted = Creature::new(false);
    rooted.set_movement_flags_runtime_like_cpp(MovementFlag::HOVER);
    rooted.unit_mut().add_unit_state(UnitState::ROOT.bits());
    let rooted_plan = rooted.set_death_state_runtime_with_fall_like_cpp(
        DeathState::JustDied,
        1_000,
        Some(CreatureDeathFallContextLikeCpp {
            is_underwater: false,
            has_valid_ground_height: true,
            vertical_delta: 12.5,
            movement_id: 77,
            duration_ms: 850,
        }),
    );
    assert!(!rooted_plan.contains(CreatureRuntimeAction::MoveFall));
}

#[test]
fn creature_owned_loot_is_looted_matches_cpp_gold_and_unlooted_count() {
    assert!(CreatureOwnedLoot::default().is_looted_like_cpp());
    assert!(!CreatureOwnedLoot::new(1, 0).is_looted_like_cpp());
    assert!(!CreatureOwnedLoot::new(0, 1).is_looted_like_cpp());
    assert!(!CreatureOwnedLoot::new(1, 1).is_looted_like_cpp());
}

#[test]
fn creature_owns_full_loot_authority_and_clear_retires_it() {
    let mut creature = Creature::new(false);
    let full_loot = CreatureLoot {
        loot_guid: ObjectGuid::EMPTY,
        coins: 17,
        unlooted_count: 2,
        loot_type: 1,
        dungeon_encounter_id: 0,
        loot_method: 0,
        loot_master: ObjectGuid::EMPTY,
        round_robin_player: ObjectGuid::EMPTY,
        player_ffa_items: Vec::new(),
        players_looting: Vec::new(),
        allowed_looters: Vec::new(),
        items: Vec::new(),
        looted_by_player: false,
    };
    creature.initialize_shared_loot_authority_like_cpp(full_loot);

    assert_eq!(
        creature.shared_loot_like_cpp(),
        Some(&CreatureOwnedLoot::new(17, 2))
    );
    assert!(!creature.loot_authority_like_cpp().is_retired_like_cpp());
    creature.clear_loot_like_cpp();
    assert!(creature.loot_authority_like_cpp().is_retired_like_cpp());
    assert!(
        creature
            .loot_authority_like_cpp()
            .shared_snapshot_like_cpp()
            .is_none()
    );
}

#[test]
fn creature_rebind_retires_displaced_authority_and_its_lease() {
    let player = ObjectGuid::create_player(1, 77);
    let displaced = OwnedLootAuthority::new();
    displaced.replace_like_cpp(
        Some(owned_loot_fixture_like_cpp(17, 0, vec![player])),
        HashMap::new(),
    );
    let mut creature = Creature::new(false);
    assert!(creature.rebind_loot_authority_like_cpp(displaced.clone()));
    let lease = poll_immediately_ready(displaced.reserve_money_like_cpp(player))
        .expect("the allowed player reserves the displaced authority");

    let replacement = OwnedLootAuthority::new();
    replacement.replace_like_cpp(
        Some(owned_loot_fixture_like_cpp(23, 0, vec![player])),
        HashMap::new(),
    );
    assert!(creature.rebind_loot_authority_like_cpp(replacement.clone()));

    assert!(displaced.is_retired_like_cpp());
    assert!(
        creature
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&replacement)
    );
    assert_eq!(
        lease.commit_like_cpp(),
        Err(wow_loot::LootClaimCommitError::StaleGeneration),
        "a lease against the displaced Arc must not commit after rebind"
    );
    assert_eq!(
        replacement.shared_snapshot_like_cpp().unwrap().loot.coins,
        23
    );
}

#[test]
fn creature_fully_looted_reads_active_authority_without_summary_refresh() {
    let authority = OwnedLootAuthority::new();
    authority.replace_like_cpp(
        Some(owned_loot_fixture_like_cpp(17, 0, Vec::new())),
        HashMap::new(),
    );
    let mut creature = Creature::new(false);
    creature.rebind_loot_authority_like_cpp(authority.clone());
    assert_eq!(
        creature.shared_loot_like_cpp(),
        Some(&CreatureOwnedLoot::new(17, 0))
    );
    assert!(!creature.is_fully_looted_like_cpp());

    authority.replace_like_cpp(
        Some(owned_loot_fixture_like_cpp(0, 0, Vec::new())),
        HashMap::new(),
    );

    assert_eq!(
        creature.shared_loot_like_cpp(),
        Some(&CreatureOwnedLoot::new(17, 0)),
        "the compatibility summary remains deliberately stale"
    );
    assert!(
        creature.is_fully_looted_like_cpp(),
        "lifecycle decisions must read the active object-owned authority"
    );
}

#[test]
fn creature_is_fully_looted_checks_shared_and_personal_loot_like_cpp() {
    let looted_player = ObjectGuid::create_player(1, 7);
    let unlooted_player = ObjectGuid::create_player(1, 8);
    let mut creature = Creature::new(false);

    assert!(creature.is_fully_looted_like_cpp());
    assert_eq!(creature.shared_loot_like_cpp(), None);

    creature.set_shared_loot_like_cpp(CreatureOwnedLoot::new(5, 0));
    assert!(!creature.is_fully_looted_like_cpp());

    creature.set_shared_loot_like_cpp(CreatureOwnedLoot::default());
    assert!(creature.is_fully_looted_like_cpp());

    creature.set_personal_loot_like_cpp(looted_player, CreatureOwnedLoot::default());
    assert!(creature.is_fully_looted_like_cpp());
    assert_eq!(
        creature.personal_loot_like_cpp(looted_player),
        Some(&CreatureOwnedLoot::default())
    );

    creature.set_personal_loot_like_cpp(unlooted_player, CreatureOwnedLoot::new(0, 1));
    assert!(!creature.is_fully_looted_like_cpp());

    creature.set_personal_loot_like_cpp(unlooted_player, CreatureOwnedLoot::default());
    assert!(creature.is_fully_looted_like_cpp());
}

#[test]
fn creature_loot_for_player_matches_cpp_shared_vs_personal_precedence() {
    let first = ObjectGuid::create_player(1, 7);
    let second = ObjectGuid::create_player(1, 8);
    let mut creature = Creature::new(false);

    assert_eq!(creature.loot_for_player_like_cpp(first), None);

    creature.set_shared_loot_like_cpp(CreatureOwnedLoot::new(5, 0));
    assert_eq!(
        creature.loot_for_player_like_cpp(first),
        Some(&CreatureOwnedLoot::new(5, 0))
    );
    assert_eq!(
        creature.loot_for_player_like_cpp(second),
        Some(&CreatureOwnedLoot::new(5, 0))
    );

    creature.set_personal_loot_like_cpp(first, CreatureOwnedLoot::new(0, 1));
    assert_eq!(
        creature.loot_for_player_like_cpp(first),
        Some(&CreatureOwnedLoot::new(0, 1))
    );
    assert_eq!(creature.loot_for_player_like_cpp(second), None);

    creature.set_personal_loot_like_cpp(second, CreatureOwnedLoot::new(9, 0));
    assert_eq!(
        creature.loot_for_player_like_cpp(second),
        Some(&CreatureOwnedLoot::new(9, 0))
    );
}

#[test]
fn creature_clear_loot_resets_shared_and_personal_loot_like_cpp() {
    let player = ObjectGuid::create_player(1, 9);
    let mut creature = Creature::new(false);
    creature.set_shared_loot_like_cpp(CreatureOwnedLoot::new(1, 1));
    creature.set_personal_loot_like_cpp(player, CreatureOwnedLoot::new(0, 1));

    creature.clear_loot_like_cpp();

    assert_eq!(creature.shared_loot_like_cpp(), None);
    assert_eq!(creature.personal_loot_count_like_cpp(), 0);
    assert!(creature.is_fully_looted_like_cpp());
}

#[test]
fn creature_clear_personal_loot_preserves_shared_loot_like_cpp() {
    let player = ObjectGuid::create_player(1, 10);
    let mut creature = Creature::new(false);
    creature.set_shared_loot_like_cpp(CreatureOwnedLoot::new(3, 0));
    creature.set_personal_loot_like_cpp(player, CreatureOwnedLoot::new(0, 1));

    creature.clear_personal_loot_like_cpp();

    assert_eq!(
        creature.shared_loot_like_cpp(),
        Some(&CreatureOwnedLoot::new(3, 0))
    );
    assert_eq!(creature.personal_loot_count_like_cpp(), 0);
    assert_eq!(
        creature.loot_for_player_like_cpp(player),
        Some(&CreatureOwnedLoot::new(3, 0))
    );
}

#[test]
fn creature_spell_focus_release_and_cancel_match_cpp_state_transitions() {
    let original_target = ObjectGuid::new(1, 10);
    let cast_target = ObjectGuid::new(1, 11);
    let mut creature = Creature::new(false);
    creature.unit_mut().set_target(original_target);
    creature.set_represented_spell_focus_like_cpp(700, cast_target, 2.5, true);

    assert_eq!(
        creature.spell_focus_state_like_cpp().target,
        original_target
    );
    assert_eq!(creature.unit().data().target, ObjectGuid::EMPTY);
    assert!(creature.unit().has_unit_state(UnitState::FOCUSING.bits()));

    creature.release_spell_focus_like_cpp(None, false, false, false);

    assert_eq!(creature.spell_focus_state_like_cpp().spell_id, None);
    assert_eq!(creature.spell_focus_state_like_cpp().delay_ms, 1);
    assert!(!creature.unit().has_unit_state(UnitState::FOCUSING.bits()));
    assert!(creature.has_spell_focus_like_cpp(None));

    creature.do_not_reacquire_spell_focus_target_like_cpp();

    assert_eq!(creature.spell_focus_state_like_cpp().spell_id, None);
    assert_eq!(creature.spell_focus_state_like_cpp().delay_ms, 0);
    assert!(!creature.has_spell_focus_like_cpp(None));
}
