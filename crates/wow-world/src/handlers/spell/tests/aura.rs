//! Spell handler aura scenarios.
//!
//! Split out of the inline test module under #624; assertions unchanged.

use super::*;

#[tokio::test]
async fn cancel_channelling_no_aura_cancel_spell_preserves_channel_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let cast_id = ObjectGuid::create_world_object(HighGuid::Cast, 0, 1, 0, 0, 1, 12);
    install_canonical_player(&mut session, &canonical, player_guid);
    session.set_spell_store(mounted_spell_store_with_no_aura_cancel(12_345, 0));
    install_active_spell_cast(&mut session, 12_345, cast_id);
    let spell = install_canonical_channeled_spell(&mut session, player_guid, 12_345);

    session
        .handle_cancel_channelling(cancel_channelling_packet(12_345, 40))
        .await;

    assert_eq!(
        canonical_channeled_spell_id(&mut session),
        Some(spell.spell_id)
    );
    assert_eq!(
        session
            .active_spell_cast_snapshot_like_cpp()
            .as_ref()
            .map(|active_cast| active_cast.spell_id),
        Some(12_345)
    );
    assert!(send_rx.is_empty());
}
#[tokio::test]
async fn cancel_aura_without_matching_represented_aura_stays_silent_like_cpp() {
    let (mut session, send_rx) = make_session();
    let caster_guid = ObjectGuid::create_player(1, 42);

    session
        .handle_cancel_aura(cancel_aura_packet(12_345, caster_guid))
        .await;

    assert!(send_rx.is_empty());
}
#[tokio::test]
async fn cancel_aura_channeled_spell_interrupts_matching_channel_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let cast_id = ObjectGuid::create_world_object(HighGuid::Cast, 0, 1, 0, 0, 1, 13);
    install_canonical_player(&mut session, &canonical, player_guid);
    session.set_spell_store(channeled_spell_store(12_345));
    install_active_spell_cast(&mut session, 12_345, cast_id);
    install_canonical_channeled_spell(&mut session, player_guid, 12_345);

    session
        .handle_cancel_aura(cancel_aura_packet(12_345, ObjectGuid::EMPTY))
        .await;

    assert_eq!(canonical_channeled_spell_id(&mut session), None);
    assert!(session.active_spell_cast_snapshot_like_cpp().is_none());
    assert!(send_rx.is_empty());
}
#[tokio::test]
async fn cancel_aura_channeled_mismatched_current_spell_preserves_channel_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let cast_id = ObjectGuid::create_world_object(HighGuid::Cast, 0, 1, 0, 0, 1, 14);
    install_canonical_player(&mut session, &canonical, player_guid);
    session.set_spell_store(channeled_spell_store(67_890));
    install_active_spell_cast(&mut session, 12_345, cast_id);
    let spell = install_canonical_channeled_spell(&mut session, player_guid, 12_345);

    session
        .handle_cancel_aura(cancel_aura_packet(67_890, ObjectGuid::EMPTY))
        .await;

    assert_eq!(
        canonical_channeled_spell_id(&mut session),
        Some(spell.spell_id)
    );
    assert_eq!(
        session
            .active_spell_cast_snapshot_like_cpp()
            .as_ref()
            .map(|active_cast| active_cast.spell_id),
        Some(12_345)
    );
    assert!(send_rx.is_empty());
}
#[tokio::test]
async fn cancel_aura_channeled_no_aura_cancel_preserves_channel_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let cast_id = ObjectGuid::create_world_object(HighGuid::Cast, 0, 1, 0, 0, 1, 15);
    install_canonical_player(&mut session, &canonical, player_guid);
    session.set_spell_store(channeled_spell_store_with_no_aura_cancel(12_345));
    install_active_spell_cast(&mut session, 12_345, cast_id);
    let spell = install_canonical_channeled_spell(&mut session, player_guid, 12_345);

    session
        .handle_cancel_aura(cancel_aura_packet(12_345, ObjectGuid::EMPTY))
        .await;

    assert_eq!(
        canonical_channeled_spell_id(&mut session),
        Some(spell.spell_id)
    );
    assert_eq!(
        session
            .active_spell_cast_snapshot_like_cpp()
            .as_ref()
            .map(|active_cast| active_cast.spell_id),
        Some(12_345)
    );
    assert!(send_rx.is_empty());
}
#[tokio::test]
async fn cancel_growth_aura_removes_represented_mod_scale_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));
    session.set_spell_store(mod_scale_spell_store(12_345, false));

    session
        .execute_spell(12_345, player_guid)
        .await
        .expect("represented mod-scale aura should apply");
    let _ = drain_server_opcodes(&send_rx);
    assert!(
        session.visible_auras.values().any(|aura| {
            aura.represented_effect == Some(RepresentedAuraEffectLikeCpp::ModScale)
        })
    );

    session
        .handle_cancel_growth_aura(WorldPacket::new_empty())
        .await;

    assert!(
        !session.visible_auras.values().any(|aura| {
            aura.represented_effect == Some(RepresentedAuraEffectLikeCpp::ModScale)
        })
    );
}
#[tokio::test]
async fn cancel_growth_aura_no_aura_cancel_preserves_mod_scale_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));
    session.set_spell_store(mod_scale_spell_store(12_345, true));

    session
        .execute_spell(12_345, player_guid)
        .await
        .expect("represented no-aura-cancel mod-scale aura should apply");
    let _ = drain_server_opcodes(&send_rx);

    session
        .handle_cancel_growth_aura(WorldPacket::new_empty())
        .await;

    assert!(
        session.visible_auras.values().any(|aura| {
            aura.represented_effect == Some(RepresentedAuraEffectLikeCpp::ModScale)
        })
    );
}
#[tokio::test]
async fn cancel_mod_speed_no_control_removes_matching_mover_aura_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));
    session.set_spell_store(mod_speed_no_control_spell_store(12_345, false));

    session
        .execute_spell(12_345, player_guid)
        .await
        .expect("represented mod-speed-no-control aura should apply");
    let _ = drain_server_opcodes(&send_rx);
    assert!(session.visible_auras.values().any(|aura| {
        aura.represented_effect == Some(RepresentedAuraEffectLikeCpp::ModSpeedNoControl)
    }));

    assert!(
        session
            .try_handle_cancel_mod_speed_no_control_auras_like_cpp(
                cancel_mod_speed_no_control_packet(player_guid),
            )
            .await
    );

    assert!(!session.visible_auras.values().any(|aura| {
        aura.represented_effect == Some(RepresentedAuraEffectLikeCpp::ModSpeedNoControl)
    }));
}
#[tokio::test]
async fn cancel_mod_speed_no_control_no_aura_cancel_preserves_aura_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));
    session.set_spell_store(mod_speed_no_control_spell_store(12_345, true));

    session
        .execute_spell(12_345, player_guid)
        .await
        .expect("represented no-aura-cancel mod-speed-no-control aura should apply");
    let _ = drain_server_opcodes(&send_rx);

    assert!(
        session
            .try_handle_cancel_mod_speed_no_control_auras_like_cpp(
                cancel_mod_speed_no_control_packet(player_guid),
            )
            .await
    );

    assert!(session.visible_auras.values().any(|aura| {
        aura.represented_effect == Some(RepresentedAuraEffectLikeCpp::ModSpeedNoControl)
    }));
}
#[tokio::test]
async fn pet_cancel_aura_removes_owned_pet_aura_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let pet_guid = ObjectGuid::create_world_object(HighGuid::Pet, 0, 1, 571, 0, 777, 42);
    let spell_id = 12_345;
    let aura = AppliedAuraRef::new(spell_id, ObjectGuid::EMPTY, 0, 0x1);
    install_canonical_player(&mut session, &canonical, player_guid);
    session.set_spell_store(basic_spell_store([spell_id as i32]));
    set_canonical_player_pet_guid(&canonical, player_guid, pet_guid);
    add_canonical_test_pet_on_map(&canonical, player_guid, pet_guid, spell_id, true);

    session
        .handle_pet_cancel_aura(pet_cancel_aura_packet(pet_guid, spell_id))
        .await;

    assert!(!canonical_pet_has_applied_aura(&canonical, pet_guid, aura));
    assert!(send_rx.is_empty());
}
#[tokio::test]
async fn pet_cancel_aura_missing_spellinfo_preserves_pet_aura_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let pet_guid = ObjectGuid::create_world_object(HighGuid::Pet, 0, 1, 571, 0, 777, 43);
    let spell_id = 12_346;
    let aura = AppliedAuraRef::new(spell_id, ObjectGuid::EMPTY, 0, 0x1);
    install_canonical_player(&mut session, &canonical, player_guid);
    session.set_spell_store(basic_spell_store([]));
    set_canonical_player_pet_guid(&canonical, player_guid, pet_guid);
    add_canonical_test_pet_on_map(&canonical, player_guid, pet_guid, spell_id, true);

    session
        .handle_pet_cancel_aura(pet_cancel_aura_packet(pet_guid, spell_id))
        .await;

    assert!(canonical_pet_has_applied_aura(&canonical, pet_guid, aura));
    assert!(send_rx.is_empty());
}
#[tokio::test]
async fn pet_cancel_aura_non_owned_pet_preserves_aura_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let other_player_guid = ObjectGuid::create_player(1, 43);
    let pet_guid = ObjectGuid::create_world_object(HighGuid::Pet, 0, 1, 571, 0, 777, 44);
    let spell_id = 12_347;
    let aura = AppliedAuraRef::new(spell_id, ObjectGuid::EMPTY, 0, 0x1);
    install_canonical_player(&mut session, &canonical, player_guid);
    session.set_spell_store(basic_spell_store([spell_id as i32]));
    add_canonical_test_pet_on_map(&canonical, other_player_guid, pet_guid, spell_id, true);

    session
        .handle_pet_cancel_aura(pet_cancel_aura_packet(pet_guid, spell_id))
        .await;

    assert!(canonical_pet_has_applied_aura(&canonical, pet_guid, aura));
    assert!(send_rx.is_empty());
}
#[tokio::test]
async fn pet_cancel_aura_dead_pet_sends_feedback_and_preserves_aura_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let pet_guid = ObjectGuid::create_world_object(HighGuid::Pet, 0, 1, 571, 0, 777, 45);
    let spell_id = 12_348;
    let aura = AppliedAuraRef::new(spell_id, ObjectGuid::EMPTY, 0, 0x1);
    install_canonical_player(&mut session, &canonical, player_guid);
    session.set_spell_store(basic_spell_store([spell_id as i32]));
    set_canonical_player_pet_guid(&canonical, player_guid, pet_guid);
    add_canonical_test_pet_on_map(&canonical, player_guid, pet_guid, spell_id, false);

    session
        .handle_pet_cancel_aura(pet_cancel_aura_packet(pet_guid, spell_id))
        .await;

    assert!(canonical_pet_has_applied_aura(&canonical, pet_guid, aura));
    let bytes = send_rx.try_recv().expect("pet action feedback");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::PetActionFeedback as u16
    );
    assert_eq!(&bytes[2..6], &0i32.to_le_bytes());
    assert_eq!(
        bytes[6],
        wow_packet::packets::pet::PET_ACTION_FEEDBACK_DEAD_LIKE_CPP
    );
    assert!(send_rx.is_empty());
}
#[tokio::test]
async fn pet_cancel_aura_removes_charmed_creature_aura_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let creature_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 46);
    let spell_id = 12_349;
    let aura = AppliedAuraRef::new(spell_id, ObjectGuid::EMPTY, 0, 0x1);
    install_canonical_player(&mut session, &canonical, player_guid);
    session.set_spell_store(basic_spell_store([spell_id as i32]));
    set_canonical_player_charmed_guid(&canonical, player_guid, creature_guid);
    add_canonical_test_creature_on_map(
        &canonical,
        creature_guid,
        Position::new(11.0, 21.0, 30.0, 0.0),
        571,
        0,
        false,
    );
    {
        let mut guard = canonical.lock().unwrap();
        let creature = guard
            .find_map_mut(571, 0)
            .unwrap()
            .map_mut()
            .get_typed_creature_mut(creature_guid)
            .unwrap();
        creature.unit_mut().set_max_health(100);
        creature.unit_mut().set_health(100);
        creature.unit_mut().subsystems_mut().auras.add_applied(aura);
    }

    session
        .handle_pet_cancel_aura(pet_cancel_aura_packet(creature_guid, spell_id))
        .await;

    assert!(!canonical_creature_has_applied_aura(
        &canonical,
        creature_guid,
        aura
    ));
    assert!(send_rx.is_empty());
}
