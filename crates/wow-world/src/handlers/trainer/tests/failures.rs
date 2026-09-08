use super::*;

#[tokio::test]
async fn definite_trainer_commit_failure_never_charges_grants_or_publishes() {
    let mut fixture = trainer_fixture();
    fixture
        .session
        .set_player_trainer_interaction_like_cpp(fixture.trainer, DEFAULT_TRAINER_ID);
    fixture.session.set_player_gold_like_cpp(100);
    fixture
        .session
        .set_loot_money_persistence_test_result_like_cpp(false);

    fixture
        .session
        .handle_trainer_buy_spell(trainer_buy_packet(
            fixture.trainer,
            DEFAULT_TRAINER_ID as i32,
            AVAILABLE_TRAINER_SPELL,
        ))
        .await;

    assert_eq!(fixture.session.player_gold_like_cpp(), 100);
    assert!(
        !fixture
            .session
            .known_spells_like_cpp()
            .contains(&AVAILABLE_TRAINER_SPELL)
    );
    assert!(fixture.send_rx.try_recv().is_err());
}

#[tokio::test]
async fn missing_character_database_never_charges_grants_or_publishes() {
    let mut fixture = trainer_fixture();
    fixture
        .session
        .set_player_trainer_interaction_like_cpp(fixture.trainer, DEFAULT_TRAINER_ID);
    fixture.session.set_player_gold_like_cpp(100);

    fixture
        .session
        .handle_trainer_buy_spell(trainer_buy_packet(
            fixture.trainer,
            DEFAULT_TRAINER_ID as i32,
            AVAILABLE_TRAINER_SPELL,
        ))
        .await;

    assert_eq!(fixture.session.player_gold_like_cpp(), 100);
    assert!(
        !fixture
            .session
            .known_spells_like_cpp()
            .contains(&AVAILABLE_TRAINER_SPELL)
    );
    assert!(fixture.send_rx.try_recv().is_err());
}

#[tokio::test]
async fn insufficient_money_uses_prepared_effective_price_without_mutation() {
    let mut fixture = trainer_fixture();
    fixture
        .session
        .set_player_trainer_interaction_like_cpp(fixture.trainer, DEFAULT_TRAINER_ID);
    fixture.session.set_player_gold_like_cpp(19);

    fixture
        .session
        .handle_trainer_buy_spell(trainer_buy_packet(
            fixture.trainer,
            DEFAULT_TRAINER_ID as i32,
            AVAILABLE_TRAINER_SPELL,
        ))
        .await;

    assert_eq!(fixture.session.player_gold_like_cpp(), 19);
    assert_eq!(
        fixture.send_rx.try_recv().unwrap(),
        TrainerBuyFailed {
            trainer_guid: fixture.trainer,
            spell_id: AVAILABLE_TRAINER_SPELL,
            reason: 1,
        }
        .to_bytes()
    );
    assert!(fixture.send_rx.try_recv().is_err());
}

#[test]
fn buy_registration_carries_the_call_while_legacy_shortcuts_stay_disabled() {
    let trainer = include_str!("../../trainer.rs");
    let registrations: Vec<_> = inventory::iter::<PacketHandlerEntry>
        .into_iter()
        .filter(|entry| entry.opcode == ClientOpcodes::TrainerBuySpell)
        .collect();
    assert_eq!(registrations.len(), 1);
    let entry = registrations[0];
    assert_eq!(entry.handler_name, "handle_trainer_buy_spell");
    assert_eq!(entry.status, SessionStatus::LoggedIn);
    assert_eq!(entry.processing, PacketProcessing::Inplace);
    // The adjacent wire-dispatch test exercises this registered call. Avoid
    // finding a stale call string inside the test's own source as before.

    let buy = trainer
        .split("pub async fn handle_trainer_buy_spell")
        .nth(1)
        .expect("buy handler")
        .split("\n}\n\n#[cfg(test)]")
        .next()
        .expect("buy handler body");
    for forbidden in [
        "INS_CHARACTER_SPELL",
        "UPD_CHAR_MONEY",
        "set_player_gold_like_cpp",
        "learn_known_spell_like_cpp",
        "LearnedSpells::single",
    ] {
        assert!(
            !buy.contains(forbidden),
            "#159 must use the prepared atomic boundary, not legacy shortcut `{forbidden}`"
        );
    }
}

#[tokio::test]
async fn trainer_buy_wire_dispatch_reaches_handler_only_when_logged_in_like_cpp() {
    const MISSING_SPELL: i32 = 99_001;

    fn trainer_buy_wire_packet(
        trainer_guid: ObjectGuid,
        trainer_id: i32,
        spell_id: i32,
    ) -> WorldPacket {
        let mut packet = WorldPacket::new_empty();
        packet.write_uint16(ClientOpcodes::TrainerBuySpell as u16);
        packet.write_packed_guid(&trainer_guid);
        packet.write_int32(trainer_id);
        packet.write_int32(spell_id);
        packet.reset_read();
        packet
    }

    let mut logged_in = trainer_fixture();
    logged_in.session.set_state(SessionState::LoggedIn);
    logged_in
        .session
        .set_player_trainer_interaction_like_cpp(logged_in.trainer, DEFAULT_TRAINER_ID);
    logged_in
        .session
        .dispatch_packet(
            &crate::session::SessionHandlerCatalogsLikeCpp::default(),
            trainer_buy_wire_packet(logged_in.trainer, DEFAULT_TRAINER_ID as i32, MISSING_SPELL),
        )
        .await;
    assert_eq!(
        logged_in.send_rx.try_recv().expect("TrainerBuyFailed"),
        TrainerBuyFailed {
            trainer_guid: logged_in.trainer,
            spell_id: MISSING_SPELL,
            reason: 0,
        }
        .to_bytes()
    );
    assert!(logged_in.send_rx.try_recv().is_err());

    let mut authed = trainer_fixture();
    authed
        .session
        .set_player_trainer_interaction_like_cpp(authed.trainer, DEFAULT_TRAINER_ID);
    authed
        .session
        .dispatch_packet(
            &crate::session::SessionHandlerCatalogsLikeCpp::default(),
            trainer_buy_wire_packet(authed.trainer, DEFAULT_TRAINER_ID as i32, MISSING_SPELL),
        )
        .await;
    assert!(
        authed.send_rx.try_recv().is_err(),
        "LoggedIn metadata must reject TrainerBuySpell while the session is Authed"
    );
}

#[tokio::test]
async fn valid_trainer_mismatch_removes_feign_before_silent_reject_like_cpp() {
    let mut fixture = trainer_fixture();
    fixture
        .session
        .set_player_trainer_interaction_like_cpp(fixture.trainer, DEFAULT_TRAINER_ID);
    seed_feign_death(&mut fixture.session, 6);

    fixture
        .session
        .handle_trainer_buy_spell(trainer_buy_packet(
            fixture.other_trainer,
            DEFAULT_TRAINER_ID as i32,
            KNOWN_TRAINER_SPELL,
        ))
        .await;

    assert!(
        fixture.send_rx.try_recv().is_ok(),
        "removing feign death publishes its aura update"
    );
    assert!(fixture.send_rx.try_recv().is_err());
    assert!(
        !fixture
            .session
            .resolved_player_visible_auras_like_cpp()
            .expect("canonical Player aura owner")
            .contains_key(&6)
    );
    assert!(!canonical_player_has_died_state(&mut fixture.session));
    assert!(
        fixture.session.player_trainer_interaction_matches_like_cpp(
            fixture.trainer,
            DEFAULT_TRAINER_ID as i32
        )
    );
}

#[tokio::test]
async fn invalid_trainer_does_not_remove_feign_or_binding_like_cpp() {
    let mut fixture = trainer_fixture();
    fixture
        .session
        .set_player_trainer_interaction_like_cpp(fixture.trainer, DEFAULT_TRAINER_ID);
    seed_feign_death(&mut fixture.session, 6);

    fixture
        .session
        .handle_trainer_buy_spell(trainer_buy_packet(
            fixture.vendor,
            DEFAULT_TRAINER_ID as i32,
            KNOWN_TRAINER_SPELL,
        ))
        .await;

    assert!(fixture.send_rx.try_recv().is_err());
    assert!(
        fixture
            .session
            .resolved_player_visible_auras_like_cpp()
            .expect("canonical Player aura owner")
            .contains_key(&6)
    );
    assert!(canonical_player_has_died_state(&mut fixture.session));
    assert!(
        fixture.session.player_trainer_interaction_matches_like_cpp(
            fixture.trainer,
            DEFAULT_TRAINER_ID as i32
        )
    );
}
