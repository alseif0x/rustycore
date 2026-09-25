use super::*;

#[tokio::test]
async fn spell_history_loads_preserve_cpp_order_expiry_and_charge_aggregation() {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let port = AuxiliaryLoadPortLikeCpp::new([
        PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
            PlayerLoginAuxiliaryLoadedLikeCpp::SpellCooldowns(vec![
                PlayerSpellCooldownLoadRowLikeCpp {
                    spell_id: 133,
                    item_id: 6948,
                    cooldown_end: now + 60,
                    category_id: 12,
                    category_end: now + 30,
                },
                PlayerSpellCooldownLoadRowLikeCpp {
                    spell_id: 134,
                    item_id: 0,
                    cooldown_end: now - 1,
                    category_id: 0,
                    category_end: 0,
                },
            ]),
        ),
        PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
            PlayerLoginAuxiliaryLoadedLikeCpp::SpellCharges(vec![
                PlayerSpellChargeLoadRowLikeCpp {
                    category_id: 42,
                    recharge_start: now - 5,
                    recharge_end: now + 45,
                },
                PlayerSpellChargeLoadRowLikeCpp {
                    category_id: 42,
                    recharge_start: now - 10,
                    recharge_end: now + 30,
                },
                PlayerSpellChargeLoadRowLikeCpp {
                    category_id: 7,
                    recharge_start: now - 20,
                    recharge_end: now - 1,
                },
            ]),
        ),
    ]);
    let (mut session, _, _) = make_session();
    session.set_player_lifecycle_port_like_cpp(port.clone());
    let guid = ObjectGuid::create_player(1, 46);

    let (history, charges) = session
        .load_character_spell_history_packets_like_cpp(guid)
        .await;

    assert_eq!(history.len(), 1);
    assert_eq!(history[0].spell_id, 133);
    assert_eq!(history[0].item_id, 6948);
    assert_eq!(history[0].category, 12);
    assert_eq!(charges.len(), 1);
    assert_eq!(charges[0].category, 42);
    assert_eq!(charges[0].consumed_charges, 2);
    assert!(charges[0].next_recovery_time_ms > 0);
    assert!(session.represented_character_spell_cooldowns_loaded_like_cpp);
    assert!(session.represented_character_spell_charges_loaded_like_cpp);
    assert_eq!(
        session.represented_character_spell_cooldowns_like_cpp.len(),
        1
    );
    assert_eq!(
        session.represented_character_spell_charges_like_cpp[&42].len(),
        2
    );
    assert_eq!(
        port.requests(),
        vec![
            PlayerLoginAuxiliaryLoadRequestLikeCpp::SpellCooldowns { player_guid: 46 },
            PlayerLoginAuxiliaryLoadRequestLikeCpp::SpellCharges { player_guid: 46 },
        ]
    );
}

#[tokio::test]
async fn spell_history_cooldown_failure_does_not_suppress_independent_charges_like_cpp() {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let port = AuxiliaryLoadPortLikeCpp::new([
        PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed {
            reason: "cooldown query failed".to_owned(),
        },
        PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
            PlayerLoginAuxiliaryLoadedLikeCpp::SpellCharges(vec![
                PlayerSpellChargeLoadRowLikeCpp {
                    category_id: 42,
                    recharge_start: now,
                    recharge_end: now + 60,
                },
            ]),
        ),
    ]);
    let (mut session, _, _) = make_session();
    session.set_player_lifecycle_port_like_cpp(port);

    let (history, charges) = session
        .load_character_spell_history_packets_like_cpp(ObjectGuid::create_player(1, 47))
        .await;

    assert!(history.is_empty());
    assert_eq!(charges.len(), 1);
    assert!(!session.represented_character_spell_cooldowns_loaded_like_cpp);
    assert!(session.represented_character_spell_charges_loaded_like_cpp);
}
