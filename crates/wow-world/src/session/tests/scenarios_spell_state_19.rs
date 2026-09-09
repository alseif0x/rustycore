//! Session scenarios exercising the represented spell state responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn spell_parry_and_block_primary_fields_set_caster_flags_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let parry_spell_id = 755_i32;
    let block_spell_id = 756_i32;
    let player_guid = ObjectGuid::create_player(1, 75);
    let target_guid = ObjectGuid::create_player(1, 76);
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "PrimaryDefensiveCaster".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        parry_spell_id,
        wow_data::SpellInfo {
            spell_id: parry_spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_PARRY,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: Vec::new(),
        },
    );
    spell_store.insert(
        block_spell_id,
        wow_data::SpellInfo {
            spell_id: block_spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_BLOCK,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: Vec::new(),
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(parry_spell_id, target_guid)
        .await
        .expect("represented primary parry spell should set caster flag");
    session
        .execute_spell(block_spell_id, target_guid)
        .await
        .expect("represented primary block spell should set caster flag");

    assert_eq!(
        session.mutate_canonical_player_like_cpp(|player| {
            (
                player.unit().can_parry_like_cpp(),
                player.unit().can_block_like_cpp(),
            )
        }),
        Some((true, true))
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::CooldownEvent,
            ServerOpcodes::SpellGo,
            ServerOpcodes::CooldownEvent,
        ]
    );
}
#[tokio::test]
async fn spell_proficiency_effect_sends_accumulated_masks_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 77);
    session.set_player_guid(Some(player_guid));

    let spells = [
        (880_i32, ItemClass::Weapon as i8, 0x0000_0400_i32),
        (881_i32, ItemClass::Weapon as i8, 0x0000_0010_i32),
        (882_i32, ItemClass::Weapon as i8, 0x0008_0000_i32),
        (883_i32, ItemClass::Armor as i8, 0x0000_0002_i32),
        (884_i32, ItemClass::Weapon as i8, 0x0000_4000_i32),
        (885_i32, ItemClass::Armor as i8, 0x0000_0001_i32),
    ];

    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, _, _) in spells {
        spell_store.insert(
            spell_id,
            wow_data::SpellInfo {
                spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: 0,
                effect_base_points: 0,
                effect_bonus_coefficient: 0.0,
                aura_type: None,
                display_flags: 0,
                requires_spell_focus: 0,
                power_costs: Vec::new(),
                effects: vec![wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_PROFICIENCY,
                    ..Default::default()
                }],
            },
        );
    }
    session.set_spell_store(Arc::new(spell_store));
    session.set_spell_equipped_items_store(Arc::new(SpellEquippedItemsStore::from_entries(
        spells.into_iter().enumerate().map(
            |(idx, (spell_id, equipped_item_class, equipped_item_subclass))| {
                SpellEquippedItemsEntry {
                    id: idx as u32 + 1,
                    spell_id,
                    equipped_item_class,
                    equipped_item_inv_types: 0,
                    equipped_item_subclass,
                }
            },
        ),
    )));

    for (spell_id, _, _) in spells {
        session
            .execute_spell(spell_id, player_guid)
            .await
            .expect("represented C++ EffectProficiency should execute");
    }

    let set_proficiency_packets = drain_server_packet_bytes(&send_rx)
        .into_iter()
        .filter(|bytes| {
            bytes.len() >= 7
                && u16::from_le_bytes([bytes[0], bytes[1]]) == ServerOpcodes::SetProficiency as u16
        })
        .map(|bytes| {
            (
                u32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]),
                bytes[6],
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        set_proficiency_packets,
        vec![
            (1024, ItemClass::Weapon as u8),
            (1040, ItemClass::Weapon as u8),
            (525_328, ItemClass::Weapon as u8),
            (2, ItemClass::Armor as u8),
            (541_712, ItemClass::Weapon as u8),
            (3, ItemClass::Armor as u8),
        ]
    );
}
#[tokio::test]
async fn login_known_spell_proficiencies_send_without_spell_cast_packets_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 78);
    session.set_player_guid(Some(player_guid));

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        890,
        wow_data::SpellInfo {
            spell_id: 890,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_PROFICIENCY,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: Vec::new(),
        },
    );
    session.set_spell_store(Arc::new(spell_store));
    session.set_spell_equipped_items_store(Arc::new(SpellEquippedItemsStore::from_entries([
        SpellEquippedItemsEntry {
            id: 1,
            spell_id: 890,
            equipped_item_class: ItemClass::Weapon as i8,
            equipped_item_inv_types: 0,
            equipped_item_subclass: 0x0000_0400,
        },
    ])));

    assert_eq!(
        session.apply_login_known_spell_proficiencies_like_cpp(&[890]),
        1
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SetProficiency]
    );
}
#[test]
fn combat_tick_base_attack_casts_current_melee_spell_instead_of_damage_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_023);
    let player = ObjectGuid::create_player(1, 72);

    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player,
        "MeleeSpell".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    let melee_spell = wow_entities::CurrentSpellRef::new(12_345, Some(player), None);
    session
        .mutate_canonical_player_like_cpp(|player| {
            let unit = player.unit_mut();
            unit.set_attacking(Some(guid));
            unit.set_target(guid);
            unit.add_unit_state(UnitState::MELEE_ATTACKING.bits());
            unit.set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 2_000);
            unit.set_weapon_damage(WeaponAttackType::BaseAttack, 7.0, 7.0);
            unit.set_current_cast_spell(wow_entities::CurrentSpellSlot::Melee, melee_spell);
        })
        .unwrap();
    session.combat_target = Some(guid);
    session.in_combat = true;
    register_test_creature(&mut session, manager.clone(), guid, 40);
    session
        .mutate_world_creature(guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();

    session.tick_combat_sync();

    let hp = manager
        .read()
        .unwrap()
        .find_creature(0, 0, guid)
        .unwrap()
        .current_hp();
    assert_eq!(hp, 40);
    let guard = canonical.lock().unwrap();
    let player_entity = guard
        .find_map(0, 0)
        .unwrap()
        .map()
        .get_typed_player(player)
        .unwrap();
    assert_eq!(
        player_entity
            .unit()
            .current_spell(wow_entities::CurrentSpellSlot::Melee),
        None
    );
    assert_eq!(
        player_entity
            .unit()
            .attack_timer(WeaponAttackType::BaseAttack),
        2_000
    );
}
#[test]
fn combat_tick_removes_attacking_interrupt_auras_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_021);
    let player = ObjectGuid::create_player(1, 70);

    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player,
        "Aura".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    let aura = wow_entities::AppliedAuraRef::new(405, player, 0, 0x1);
    session
        .mutate_canonical_player_like_cpp(|player| {
            let unit = player.unit_mut();
            unit.set_attacking(Some(guid));
            unit.set_target(guid);
            unit.add_unit_state(UnitState::MELEE_ATTACKING.bits());
            unit.subsystems_mut().auras.register_applied_aura(
                aura,
                None,
                wow_entities::SPELL_AURA_INTERRUPT_FLAG_ATTACKING_LIKE_CPP,
                0,
            );
        })
        .unwrap();
    session.combat_target = Some(guid);
    session.in_combat = true;
    register_test_creature(&mut session, manager.clone(), guid, 40);
    session
        .mutate_world_creature(guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();

    session.tick_combat_sync();

    let guard = canonical.lock().unwrap();
    let player_entity = guard
        .find_map(0, 0)
        .unwrap()
        .map()
        .get_typed_player(player)
        .unwrap();
    assert!(!player_entity.unit().subsystems().auras.has_applied(aura));
}
#[test]
fn combat_tick_casting_player_skips_melee_update_without_reset_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_033);
    let player = ObjectGuid::create_player(1, 82);

    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player,
        "Caster".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    let generic_cast =
        wow_entities::CurrentSpellRef::new(12_346, Some(player), None).with_cast_time_ms(1_500);
    session
        .mutate_canonical_player_like_cpp(|player| {
            let unit = player.unit_mut();
            unit.set_attacking(Some(guid));
            unit.set_target(guid);
            unit.add_unit_state(UnitState::MELEE_ATTACKING.bits());
            unit.set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 2_000);
            unit.set_weapon_damage(WeaponAttackType::BaseAttack, 7.0, 7.0);
            unit.set_current_cast_spell(wow_entities::CurrentSpellSlot::Generic, generic_cast);
        })
        .unwrap();
    session.combat_target = Some(guid);
    session.in_combat = true;
    register_test_creature(&mut session, manager.clone(), guid, 40);
    session
        .mutate_world_creature(guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();

    session.tick_combat_sync();

    assert_eq!(
        manager
            .read()
            .unwrap()
            .find_creature(0, 0, guid)
            .unwrap()
            .current_hp(),
        40
    );
    let guard = canonical.lock().unwrap();
    let player_entity = guard
        .find_map(0, 0)
        .unwrap()
        .map()
        .get_typed_player(player)
        .unwrap();
    assert_eq!(
        player_entity
            .unit()
            .attack_timer(WeaponAttackType::BaseAttack),
        0
    );
}
#[test]
fn combat_tick_spell_delay_combat_timer_pauses_attack_timer_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_034);
    let player = ObjectGuid::create_player(1, 85);

    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player,
        "TimerPause".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    let generic_cast = wow_entities::CurrentSpellRef::new(12_347, Some(player), None)
        .with_cast_time_ms(1_500)
        .with_delay_combat_timer_during_cast(true);
    session
        .mutate_canonical_player_like_cpp(|player| {
            let unit = player.unit_mut();
            unit.set_attacking(Some(guid));
            unit.set_target(guid);
            unit.add_unit_state(UnitState::MELEE_ATTACKING.bits());
            unit.set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 2_000);
            unit.set_attack_timer(WeaponAttackType::BaseAttack, 250);
            unit.set_current_cast_spell(wow_entities::CurrentSpellSlot::Generic, generic_cast);
        })
        .unwrap();
    session.combat_tick_last_at_like_cpp = Instant::now() - std::time::Duration::from_millis(100);
    session.combat_target = Some(guid);
    session.in_combat = true;
    register_test_creature(&mut session, manager.clone(), guid, 40);

    session.tick_combat_sync();

    let guard = canonical.lock().unwrap();
    let player_entity = guard
        .find_map(0, 0)
        .unwrap()
        .map()
        .get_typed_player(player)
        .unwrap();
    assert_eq!(
        player_entity
            .unit()
            .attack_timer(WeaponAttackType::BaseAttack),
        250
    );
}
#[test]
fn player_attack_uses_stalked_aura_as_always_detectable_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let attacker = ObjectGuid::create_player(1, 59);
    let victim = ObjectGuid::create_player(1, 60);

    canonical.lock().unwrap().create_world_map(571, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        attacker,
        "Hunter".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        571,
        1,
        3,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();

    let mut victim_player = Player::new(Some(8), false);
    victim_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(victim);
    victim_player
        .unit_mut()
        .world_mut()
        .set_map(571, 0)
        .unwrap();
    victim_player
        .unit_mut()
        .world_mut()
        .relocate(Position::new(11.0, 20.0, 30.0, 0.0));
    victim_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .add_to_world();
    victim_player
        .unit_mut()
        .set_pvp_flag_like_cpp(UnitPvpFlags::PVP);
    victim_player.unit_mut().set_invisibility_like_cpp(0, 100);
    canonical
        .lock()
        .unwrap()
        .find_map_mut(571, 0)
        .unwrap()
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_player(victim_player).unwrap())
        .unwrap();

    assert_eq!(
        session.start_player_attack_like_cpp(victim),
        PlayerAttackStartLikeCppResult::Rejected
    );

    session
        .mutate_canonical_player_by_guid_like_cpp(victim, |player| {
            player
                .unit_mut()
                .subsystems_mut()
                .auras
                .register_applied_aura_type_like_cpp(
                    wow_entities::AppliedAuraRef::new(53338, attacker, 0, 0x1),
                    wow_entities::SPELL_AURA_MOD_STALKED_LIKE_CPP,
                );
        })
        .unwrap();
    assert_eq!(
        session.start_player_attack_like_cpp(victim),
        PlayerAttackStartLikeCppResult::Accepted {
            send_attack_start: true
        }
    );
}
#[tokio::test]
async fn check_area_explore_removes_indoor_outdoor_auras_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(9_101, test_spell_info_like_cpp(9_101));
    spell_store.insert(9_102, test_spell_info_like_cpp(9_102));
    spell_store.insert(9_103, test_spell_info_like_cpp(9_103));
    let mut indoor_attributes = [0; 15];
    indoor_attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_ONLY_INDOORS;
    spell_store.insert_spell_misc_attributes_like_cpp(9_101, indoor_attributes);
    let mut outdoor_attributes = [0; 15];
    outdoor_attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_ONLY_OUTDOORS;
    spell_store.insert_spell_misc_attributes_like_cpp(9_102, outdoor_attributes);
    session.set_spell_store(Arc::new(spell_store));
    session.visible_auras.insert(1, test_visible_aura(1, 9_101));
    session.visible_auras.insert(2, test_visible_aura(2, 9_102));
    session.visible_auras.insert(3, test_visible_aura(3, 9_103));

    session.set_vmap_indoor_check_like_cpp(true);
    session.set_represented_is_outdoors_like_cpp(true);

    assert!(
        !session
            .check_area_explore_and_outdoor_represented_like_cpp(0)
            .await,
        "C++ still returns after area_id==0, but aura removal already ran"
    );
    assert!(!session.visible_auras.contains_key(&1));
    assert!(session.visible_auras.contains_key(&2));
    assert!(session.visible_auras.contains_key(&3));

    session.set_represented_is_outdoors_like_cpp(false);
    assert!(
        !session
            .check_area_explore_and_outdoor_represented_like_cpp(0)
            .await
    );
    assert!(!session.visible_auras.contains_key(&2));
    assert!(session.visible_auras.contains_key(&3));
}
#[test]
fn exact_player_spell_rows_preserve_flags_and_active_only_updates_fail_closed() {
    let (mut session, _, _) = make_session();
    session.set_known_spells_like_cpp(vec![100]);
    assert!(
        session
            .complete_represented_player_spell_rows_like_cpp()
            .is_none(),
        "known_spells has no inactive, disabled, favorite or persistence-state authority"
    );

    let exact_rows = vec![
        RepresentedPlayerSpellLikeCpp {
            spell_id: 100,
            active: true,
            disabled: false,
            dependent: true,
            favorite: false,
            state: RepresentedPlayerSpellStateLikeCpp::Changed,
        },
        RepresentedPlayerSpellLikeCpp {
            spell_id: 200,
            active: true,
            disabled: true,
            dependent: false,
            favorite: true,
            state: RepresentedPlayerSpellStateLikeCpp::Unchanged,
        },
        RepresentedPlayerSpellLikeCpp {
            spell_id: 250,
            active: false,
            disabled: true,
            dependent: false,
            favorite: false,
            state: RepresentedPlayerSpellStateLikeCpp::Unchanged,
        },
        RepresentedPlayerSpellLikeCpp {
            spell_id: 300,
            active: false,
            disabled: false,
            dependent: false,
            favorite: true,
            state: RepresentedPlayerSpellStateLikeCpp::Removed,
        },
    ];
    assert!(
        session.replace_loaded_represented_player_spell_rows_like_cpp(exact_rows.clone(), false,)
    );
    assert!(session.represented_player_spell_rows_loaded_like_cpp());
    assert!(
        session
            .complete_represented_player_spell_rows_like_cpp()
            .is_none(),
        "lossless retained rows are not automatically a complete post-AddSpell snapshot"
    );

    assert!(session.set_complete_represented_player_spell_rows_like_cpp(exact_rows.clone()));
    assert_eq!(
        session
            .complete_represented_player_spell_rows_like_cpp()
            .map(|rows| rows.values().copied().collect::<Vec<_>>()),
        Some(exact_rows.clone())
    );
    assert!(
        session
            .complete_represented_player_spell_rows_like_cpp()
            .is_some_and(|rows| rows[&200].active && rows[&200].disabled),
        "disabled rows retain their persisted active bit; it is not inferred from the client projection"
    );

    session.learn_known_spell_like_cpp(200);
    assert!(
        session
            .complete_represented_player_spell_rows_like_cpp()
            .is_none(),
        "a low-level runtime learn cannot preserve authority without the full AddSpell closure"
    );

    assert!(session.set_complete_represented_spell_trait_definition_ids_like_cpp([]));
    assert!(session.set_complete_represented_override_spells_like_cpp([]));
    assert!(session.set_complete_represented_player_spell_rows_like_cpp(exact_rows));
    assert!(
        session
            .complete_represented_spell_trait_definition_ids_like_cpp()
            .is_none()
    );
    assert!(
        session
            .complete_represented_override_spells_like_cpp()
            .is_none(),
        "replacing PlayerSpellMap rows must not attribute an older auxiliary snapshot to the new map"
    );

    session.mark_represented_spell_acquisition_snapshot_complete_like_cpp();
    session.learn_known_spell_like_cpp(400);
    assert!(
        session
            .complete_represented_player_spell_rows_like_cpp()
            .is_none(),
        "partial runtime learns must fail closed instead of retaining stale auxiliary authority"
    );
    session.remove_known_spell_like_cpp(400);
    assert!(
        session
            .complete_represented_player_spell_rows_like_cpp()
            .is_none()
    );

    session.set_known_spells_like_cpp(vec![100, 400]);
    assert!(
        session
            .complete_represented_player_spell_rows_like_cpp()
            .is_none(),
        "a bulk active-only replacement must invalidate rather than fabricate exact row flags"
    );
}
#[test]
fn acquisition_runtime_preserves_inactive_has_spell_rows_after_apply_and_save_like_cpp() {
    let (mut session, _, _) = make_session();
    let rows = [
        RepresentedPlayerSpellLikeCpp {
            spell_id: 100,
            active: false,
            disabled: false,
            dependent: false,
            favorite: false,
            state: RepresentedPlayerSpellStateLikeCpp::Changed,
        },
        RepresentedPlayerSpellLikeCpp {
            spell_id: 200,
            active: true,
            disabled: false,
            dependent: false,
            favorite: false,
            state: RepresentedPlayerSpellStateLikeCpp::Changed,
        },
        RepresentedPlayerSpellLikeCpp {
            spell_id: 300,
            active: true,
            disabled: true,
            dependent: false,
            favorite: false,
            state: RepresentedPlayerSpellStateLikeCpp::Changed,
        },
        RepresentedPlayerSpellLikeCpp {
            spell_id: 400,
            active: false,
            disabled: false,
            dependent: false,
            favorite: false,
            state: RepresentedPlayerSpellStateLikeCpp::Removed,
        },
    ];

    assert!(session.replace_complete_spell_acquisition_runtime_like_cpp(
        rows,
        [],
        [],
        HashMap::new(),
        0,
        BTreeSet::new(),
    ));
    assert_eq!(
        session.known_spells_like_cpp(),
        &[100, 200],
        "C++ Player::HasSpell includes inactive non-disabled lower ranks"
    );

    session.mark_player_spells_saved_like_cpp();
    assert_eq!(
        session.known_spells_like_cpp(),
        &[100, 200],
        "normalizing PlayerSpell persistence state must not change HasSpell semantics"
    );
}
