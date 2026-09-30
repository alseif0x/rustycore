//! Talent handlers regression scenarios, part 2 of 2.
//!
//! Moved out of the talent.rs root under #662; every test is unchanged.

use super::*;

#[tokio::test]
async fn learn_talent_adds_override_spell_pair_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    let mut talent = test_talent_entry_like_cpp(101, 0, 50_101);
    talent.spell_id = 70_101;
    talent.overrides_spell_id = 60_101;
    let talent_tabs =
        install_test_talent_entries_with_tab_class_mask(&mut session, vec![talent], 1);
    session.mark_represented_talents_loaded_like_cpp();

    session
        .handle_learn_talent(&talent_tabs, learn_talent_packet(101, 0))
        .await;

    assert!(send_rx.try_recv().is_ok());
    let override_spells = session.represented_override_spells_like_cpp();
    let overrides = override_spells
        .get(&60_101)
        .expect("C++ Player::AddTalent calls AddOverrideSpell when OverridesSpellID is set");
    assert!(
        overrides.contains(&70_101),
        "C++ AddOverrideSpell stores overridden spell id -> replacement talent SpellID"
    );
}

#[tokio::test]
async fn confirm_respec_wipe_resets_active_talents_and_removes_overrides_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(3);
    let trainer = test_creature_guid(88);
    let mut talent = test_talent_entry_like_cpp(101, 0, 50_101);
    talent.spell_id = 70_101;
    talent.overrides_spell_id = 60_101;
    register_test_trainer(&mut session, trainer, NPCFlags1::TRAINER.bits());
    let talent_tabs =
        install_test_talent_entries_with_tab_class_mask(&mut session, vec![talent], 1);
    session.mark_represented_talents_loaded_like_cpp();
    session.set_player_gold_like_cpp(20_000);

    session
        .handle_learn_talent(&talent_tabs, learn_talent_packet(101, 0))
        .await;
    assert!(
        drain_sent_packets(&send_rx).iter().any(|packet| *packet
            == session
                .represented_update_talent_data_packet_like_cpp()
                .to_bytes()),
        "C++ sends SendTalentsInfoData after LearnTalent"
    );
    assert!(session.known_spells_like_cpp().contains(&50_101));
    assert!(
        session
            .represented_override_spells_like_cpp()
            .get(&60_101)
            .is_some_and(|overrides| overrides.contains(&70_101))
    );

    session
        .handle_confirm_respec_wipe(confirm_respec_wipe_packet(
            trainer,
            SPEC_RESET_TALENTS_LIKE_CPP,
        ))
        .await;

    assert!(
        drain_sent_packets(&send_rx).iter().any(|packet| *packet
            == session
                .represented_update_talent_data_packet_like_cpp()
                .to_bytes()),
        "C++ sends SendTalentsInfoData after successful ResetTalents"
    );
    assert_eq!(
        session.represented_confirm_respec_wipe_requests_like_cpp(),
        &[RepresentedConfirmRespecWipeLikeCpp {
            respec_master: trainer,
            respec_type: SPEC_RESET_TALENTS_LIKE_CPP,
        }]
    );
    assert!(
        !session.known_spells_like_cpp().contains(&50_101),
        "C++ RemoveTalent removes the active talent spell"
    );
    assert!(
        session.represented_override_spells_like_cpp().is_empty(),
        "C++ RemoveTalent calls RemoveOverrideSpell for OverridesSpellID"
    );
    let packet = session.represented_update_talent_data_packet_like_cpp();
    assert!(
        packet.groups[0].talents.is_empty(),
        "C++ ResetTalents removes talents from the active group"
    );
    assert_eq!(packet.unspent_talent_points, 71);
}

#[tokio::test]
async fn confirm_respec_wipe_removes_active_pet_not_in_slot_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(3);
    let canonical = Arc::new(Mutex::new(wow_map::MapManager::default()));
    let trainer = test_creature_guid(91);
    let player_guid = ObjectGuid::create_player(1, 42);
    let pet_guid = ObjectGuid::create_world_object(HighGuid::Pet, 0, 1, 0, 0, 500, 42);
    let position = Position::new(0.0, 0.0, 0.0, 0.0);
    register_test_trainer(&mut session, trainer, NPCFlags1::TRAINER.bits());
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
        "TalentPetRemove".to_string(),
        position,
        0,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("canonical player map");
    session.set_player_faction_template_like_cpp(35);
    add_canonical_test_trainer_like_cpp(
        &canonical,
        trainer,
        Position::new(1.0, 0.0, 0.0, 0.0),
        NPCFlags1::TRAINER.bits(),
        1,
    );
    add_canonical_test_pet_like_cpp(&canonical, pet_guid, player_guid, position);
    session.set_represented_pet_mode_state_like_cpp(
        Some(pet_guid),
        wow_packet::packets::pet::REACT_DEFENSIVE_LIKE_CPP,
        wow_packet::packets::pet::COMMAND_FOLLOW_LIKE_CPP,
    );
    session.set_represented_pet_stable_like_cpp(represented_current_pet_stable_like_cpp(42));
    session.mark_represented_talents_loaded_like_cpp();
    session.set_player_gold_like_cpp(20_000);

    session
        .handle_confirm_respec_wipe(confirm_respec_wipe_packet(
            trainer,
            SPEC_RESET_TALENTS_LIKE_CPP,
        ))
        .await;

    let _reset_update = send_rx
        .try_recv()
        .expect("C++ sends SendTalentsInfoData after successful ResetTalents");
    assert!(send_rx.try_recv().is_err());
    assert_eq!(
        session.represented_pet_guid_like_cpp(),
        None,
        "C++ ResetTalents calls RemovePet(nullptr, PET_SAVE_NOT_IN_SLOT, true)"
    );
    assert_eq!(
        session.represented_pet_stable_current_index_like_cpp(),
        None,
        "C++ Player::RemovePet resets PetStable::CurrentPetIndex for PET_SAVE_NOT_IN_SLOT"
    );
    assert_eq!(
        session.represented_temporary_unsummoned_pet_number_like_cpp(),
        0,
        "C++ talent reset does not use the temporary-unsummoned pet slot"
    );
    let manager = canonical.lock().unwrap();
    assert!(
        manager
            .find_map(0, 0)
            .unwrap()
            .map()
            .get_typed_pet(pet_guid)
            .is_none(),
        "C++ Player::RemovePet adds the live pet object to removal"
    );
}

#[tokio::test]
async fn confirm_respec_wipe_rejects_without_money_before_removing_talents_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(3);
    let trainer = test_creature_guid(89);
    let pet_guid = ObjectGuid::create_world_object(HighGuid::Pet, 0, 1, 0, 0, 500, 43);
    let mut talent = test_talent_entry_like_cpp(101, 0, 50_101);
    talent.spell_id = 70_101;
    talent.overrides_spell_id = 60_101;
    register_test_trainer(&mut session, trainer, NPCFlags1::TRAINER.bits());
    let talent_tabs =
        install_test_talent_entries_with_tab_class_mask(&mut session, vec![talent], 1);
    session.mark_represented_talents_loaded_like_cpp();

    session
        .handle_learn_talent(&talent_tabs, learn_talent_packet(101, 0))
        .await;
    let _learn_update = send_rx
        .try_recv()
        .expect("C++ sends SendTalentsInfoData after LearnTalent");
    session.set_represented_pet_mode_state_like_cpp(
        Some(pet_guid),
        wow_packet::packets::pet::REACT_DEFENSIVE_LIKE_CPP,
        wow_packet::packets::pet::COMMAND_FOLLOW_LIKE_CPP,
    );
    session.set_represented_pet_stable_like_cpp(represented_current_pet_stable_like_cpp(43));
    session.set_player_gold_like_cpp(9_999);
    session.set_represented_at_login_flags_like_cpp(
        AT_LOGIN_RENAME_LIKE_CPP | AT_LOGIN_RESET_TALENTS_LIKE_CPP,
    );

    session
        .handle_confirm_respec_wipe(confirm_respec_wipe_packet(
            trainer,
            SPEC_RESET_TALENTS_LIKE_CPP,
        ))
        .await;

    let buy_failed = send_rx
        .try_recv()
        .expect("C++ SendBuyError(BUY_ERR_NOT_ENOUGHT_MONEY) rejects ResetTalents");
    assert_eq!(
        buy_failed,
        BuyFailed {
            vendor_guid: ObjectGuid::EMPTY,
            muid: 0,
            reason: BuyResult::NotEnoughtMoney,
        }
        .to_bytes()
    );
    assert!(send_rx.try_recv().is_err());
    assert_eq!(session.player_gold_like_cpp(), 9_999);
    assert_eq!(session.represented_talent_reset_cost_like_cpp(), Some(0));
    assert_eq!(
        session.represented_talent_reset_time_secs_like_cpp(),
        Some(0)
    );
    assert_eq!(
        session.represented_talent_reset_script_hooks_like_cpp(),
        &[RepresentedTalentResetScriptHookLikeCpp { no_cost: false }],
        "C++ calls OnPlayerTalentsReset before the money gate"
    );
    assert_eq!(
        session.represented_at_login_flags_like_cpp(),
        AT_LOGIN_RENAME_LIKE_CPP,
        "C++ removes only AT_LOGIN_RESET_TALENTS before the money gate"
    );
    assert_eq!(
        session.represented_at_login_flag_removals_like_cpp(),
        &[RepresentedAtLoginFlagRemovalLikeCpp {
            flags: AT_LOGIN_RESET_TALENTS_LIKE_CPP,
            persist: true,
            db_statement_unrepresented: true,
        }],
        "C++ RemoveAtLoginFlag(AT_LOGIN_RESET_TALENTS, true) executes CHAR_UPD_REM_AT_LOGIN_FLAG before ResetTalents can fail"
    );
    assert!(
        session
            .represented_talent_respec_criteria_events_like_cpp()
            .is_empty(),
        "C++ returns before criteria updates when the money gate fails"
    );
    assert!(
        session
            .represented_confirm_respec_wipe_requests_like_cpp()
            .is_empty()
    );
    assert!(
        session
            .represented_talent_respec_visual_spell_casts_like_cpp()
            .is_empty(),
        "C++ returns before the 14867 visual cast when ResetTalents fails"
    );
    assert!(
        session.known_spells_like_cpp().contains(&50_101),
        "C++ checks money before RemoveTalent"
    );
    assert_eq!(
        session.represented_pet_guid_like_cpp(),
        Some(pet_guid),
        "C++ returns before RemovePet when ResetTalents fails the money gate"
    );
    assert_eq!(
        session.represented_pet_stable_current_index_like_cpp(),
        Some(0),
        "C++ leaves the current pet slot untouched when ResetTalents returns false"
    );
    assert!(
        session
            .represented_override_spells_like_cpp()
            .get(&60_101)
            .is_some_and(|overrides| overrides.contains(&70_101)),
        "C++ keeps override spells when ResetTalents returns false"
    );
}

#[tokio::test]
async fn confirm_respec_wipe_definite_rollback_publishes_no_covered_runtime_or_packets() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    let trainer = test_creature_guid(189);
    let pet_guid = ObjectGuid::create_world_object(HighGuid::Pet, 0, 1, 0, 0, 500, 143);
    let mut talent = test_talent_entry_like_cpp(101, 0, 50_101);
    talent.spell_id = 70_101;
    talent.overrides_spell_id = 60_101;
    register_test_trainer(&mut session, trainer, NPCFlags1::TRAINER.bits());
    let talent_tabs =
        install_test_talent_entries_with_tab_class_mask(&mut session, vec![talent], 1);
    session.mark_represented_talents_loaded_like_cpp();

    session
        .handle_learn_talent(&talent_tabs, learn_talent_packet(101, 0))
        .await;
    let _ = drain_sent_packets(&send_rx);
    session.set_represented_pet_mode_state_like_cpp(
        Some(pet_guid),
        wow_packet::packets::pet::REACT_DEFENSIVE_LIKE_CPP,
        wow_packet::packets::pet::COMMAND_FOLLOW_LIKE_CPP,
    );
    session.set_represented_pet_stable_like_cpp(represented_current_pet_stable_like_cpp(143));
    session.set_player_gold_like_cpp(20_000);
    session.set_represented_at_login_flags_like_cpp(
        AT_LOGIN_RENAME_LIKE_CPP | AT_LOGIN_RESET_TALENTS_LIKE_CPP,
    );
    session.set_loot_money_persistence_test_result_like_cpp(false);

    session
        .handle_confirm_respec_wipe(confirm_respec_wipe_packet(
            trainer,
            SPEC_RESET_TALENTS_LIKE_CPP,
        ))
        .await;

    assert!(
        send_rx.try_recv().is_err(),
        "a definite SQL rollback must not publish talent, spell, money, or visual packets"
    );
    assert_eq!(session.player_gold_like_cpp(), 20_000);
    assert_eq!(session.represented_talent_reset_cost_like_cpp(), Some(0));
    assert_eq!(
        session.represented_talent_reset_time_secs_like_cpp(),
        Some(0)
    );
    assert!(session.known_spells_like_cpp().contains(&50_101));
    assert_eq!(
        session
            .represented_update_talent_data_packet_like_cpp()
            .groups[0]
            .talents,
        vec![wow_packet::packets::misc::TalentInfoLikeCpp {
            talent_id: 101,
            rank: 0,
        }]
    );
    assert_eq!(session.represented_pet_guid_like_cpp(), Some(pet_guid));
    assert_eq!(
        session.represented_pet_stable_current_index_like_cpp(),
        Some(0)
    );
    assert!(
        session
            .represented_talent_respec_criteria_events_like_cpp()
            .is_empty()
    );
    assert!(
        session
            .represented_confirm_respec_wipe_requests_like_cpp()
            .is_empty()
    );
    assert!(
        session
            .represented_talent_respec_visual_spell_casts_like_cpp()
            .is_empty()
    );

    assert_eq!(
        session.represented_talent_reset_script_hooks_like_cpp(),
        &[RepresentedTalentResetScriptHookLikeCpp { no_cost: false }],
        "C++ fires the script hook before the reset's money/transaction gate"
    );
    assert_eq!(
        session.represented_at_login_flags_like_cpp(),
        AT_LOGIN_RENAME_LIKE_CPP,
        "C++ removes AT_LOGIN_RESET_TALENTS before entering the fallible save"
    );
}

#[tokio::test]
async fn confirm_respec_wipe_removes_feign_death_before_reset_talents_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    let trainer = test_creature_guid(90);
    let feign_slot = 7;
    register_test_trainer(&mut session, trainer, NPCFlags1::TRAINER.bits());
    let canonical = Arc::new(Mutex::new(wow_map::MapManager::default()));
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
        ObjectGuid::create_player(1, 42),
        "TalentTester".to_string(),
        Position::new(0.0, 0.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("canonical player map");
    session.set_player_faction_template_like_cpp(35);
    session.set_player_gold_like_cpp(9_999);
    session.mark_represented_talents_loaded_like_cpp();
    add_canonical_test_trainer_like_cpp(
        &canonical,
        trainer,
        Position::new(1.0, 0.0, 0.0, 0.0),
        NPCFlags1::TRAINER.bits(),
        1,
    );
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().add_unit_state(UnitState::DIED.bits());
        })
        .expect("canonical player");
    let mut feign_death = visible_aura(feign_slot, 0);
    feign_death.spell_id = 5384;
    feign_death.represented_effect = Some(RepresentedAuraEffectLikeCpp::FeignDeath);
    assert!(session.insert_player_visible_aura_like_cpp(feign_death));

    session
        .handle_confirm_respec_wipe(confirm_respec_wipe_packet(
            trainer,
            SPEC_RESET_TALENTS_LIKE_CPP,
        ))
        .await;

    let _aura_removed = send_rx
        .try_recv()
        .expect("C++ removes fake death before ResetTalents can fail");
    let buy_failed = send_rx
        .try_recv()
        .expect("C++ then sends the insufficient-money ResetTalents failure");
    assert_eq!(
        buy_failed,
        BuyFailed {
            vendor_guid: ObjectGuid::EMPTY,
            muid: 0,
            reason: BuyResult::NotEnoughtMoney,
        }
        .to_bytes()
    );
    assert!(send_rx.try_recv().is_err());
    assert!(
        !session
            .resolved_player_visible_auras_like_cpp()
            .expect("canonical Player aura owner")
            .contains_key(&feign_slot)
    );
    assert_eq!(
        session
            .mutate_canonical_player_like_cpp(|player| {
                player.unit().has_unit_state(UnitState::DIED.bits())
            })
            .expect("canonical player"),
        false,
        "C++ RemoveAurasByType(SPELL_AURA_FEIGN_DEATH) clears UNIT_STATE_DIED before ResetTalents"
    );
    assert_eq!(session.player_gold_like_cpp(), 9_999);
    assert!(
        session
            .represented_confirm_respec_wipe_requests_like_cpp()
            .is_empty(),
        "C++ returns from failed ResetTalents before recording the accepted represented reset"
    );
}

#[test]
fn represented_next_reset_talents_cost_matches_cpp_branches() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    let month = 30 * 24 * 60 * 60;
    let now = 10 * month;

    session.set_represented_talent_reset_state_like_cpp(0, now);
    assert_eq!(
        session.represented_next_reset_talents_cost_like_cpp(now),
        Some(10_000)
    );

    session.set_represented_talent_reset_state_like_cpp(10_000, now);
    assert_eq!(
        session.represented_next_reset_talents_cost_like_cpp(now),
        Some(50_000)
    );

    session.set_represented_talent_reset_state_like_cpp(50_000, now);
    assert_eq!(
        session.represented_next_reset_talents_cost_like_cpp(now),
        Some(100_000)
    );

    session.set_represented_talent_reset_state_like_cpp(100_000, now);
    assert_eq!(
        session.represented_next_reset_talents_cost_like_cpp(now),
        Some(150_000)
    );

    session.set_represented_talent_reset_state_like_cpp(500_000, now);
    assert_eq!(
        session.represented_next_reset_talents_cost_like_cpp(now),
        Some(500_000)
    );

    session.set_represented_talent_reset_state_like_cpp(500_000, now - month);
    assert_eq!(
        session.represented_next_reset_talents_cost_like_cpp(now),
        Some(450_000)
    );

    session.set_represented_talent_reset_state_like_cpp(100_000, 0);
    assert_eq!(
        session.represented_next_reset_talents_cost_like_cpp(now),
        Some(100_000)
    );
}

#[tokio::test]
async fn confirm_respec_wipe_rejects_non_talent_reset_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    let trainer = test_creature_guid(78);
    register_test_trainer(&mut session, trainer, NPCFlags1::TRAINER.bits());

    session
        .handle_confirm_respec_wipe(confirm_respec_wipe_packet(
            trainer,
            wow_packet::packets::talent::SPEC_RESET_PET_TALENTS_LIKE_CPP,
        ))
        .await;

    assert!(
        session
            .represented_confirm_respec_wipe_requests_like_cpp()
            .is_empty()
    );
}

#[tokio::test]
async fn confirm_respec_wipe_rejects_non_trainer_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    let vendor = test_creature_guid(79);
    register_test_trainer(&mut session, vendor, NPCFlags1::VENDOR.bits());

    session
        .handle_confirm_respec_wipe(confirm_respec_wipe_packet(
            vendor,
            SPEC_RESET_TALENTS_LIKE_CPP,
        ))
        .await;

    assert!(
        session
            .represented_confirm_respec_wipe_requests_like_cpp()
            .is_empty()
    );
}

#[tokio::test]
async fn confirm_respec_wipe_rejects_low_level_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    let trainer = test_creature_guid(80);
    register_test_trainer(&mut session, trainer, NPCFlags1::TRAINER.bits());
    session.set_player_level_like_cpp(14);

    session
        .handle_confirm_respec_wipe(confirm_respec_wipe_packet(
            trainer,
            SPEC_RESET_TALENTS_LIKE_CPP,
        ))
        .await;

    assert!(
        session
            .represented_confirm_respec_wipe_requests_like_cpp()
            .is_empty()
    );
}

#[tokio::test]
async fn confirm_respec_wipe_rejects_mismatched_trainer_class_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    let trainer = test_creature_guid(81);
    register_test_trainer_with_class(&mut session, trainer, NPCFlags1::TRAINER.bits(), 2);
    session.set_player_class_like_cpp(1);
    session.mark_represented_talents_loaded_like_cpp();
    session.set_player_gold_like_cpp(20_000);

    session
        .handle_confirm_respec_wipe(confirm_respec_wipe_packet(
            trainer,
            SPEC_RESET_TALENTS_LIKE_CPP,
        ))
        .await;

    assert!(
        session
            .represented_confirm_respec_wipe_requests_like_cpp()
            .is_empty(),
        "C++ Creature::CanResetTalents requires player class to match creature_template.trainer_class"
    );
}
