use super::*;

#[tokio::test]
async fn represented_haste_aura_scales_attack_time_multiplier_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 76);
    session.player_guid = Some(player_guid);
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 0, 0);
    let mut spell_store = wow_data::SpellStore::new();
    // `SPELL_AURA_MOD_MELEE_HASTE` and `SPELL_AURA_MOD_SPEED_SLOW_ALL` with
    // opposite signs exercise both branches of `ApplyAttackTimePercentMod`.
    spell_store.insert(
        90_991,
        represented_aura_spell_like_cpp(
            90_991,
            wow_data::spell::aura_types::SPELL_AURA_MOD_MELEE_HASTE,
            0,
            30,
        ),
    );
    spell_store.insert(
        90_992,
        represented_aura_spell_like_cpp(
            90_992,
            wow_data::spell::aura_types::SPELL_AURA_MOD_SPEED_SLOW_ALL,
            0,
            -30,
        ),
    );
    session.set_spell_store(Arc::new(spell_store));

    assert_eq!(
        session
            .canonical_player_snapshot_like_cpp(|player| player.unit().mod_attack_speed_pct())
            .expect("canonical player"),
        [1.0, 1.0, 1.0]
    );

    session
        .apply_aura(90_991, player_guid, 30_000, 1)
        .expect("apply melee haste aura");
    // `100 / (100 + 30)` for main hand and off hand, ranged untouched. The
    // consumer is the swing timer reset, which reads the same multiplier.
    let hasted = session
        .mutate_canonical_player_like_cpp(|player| {
            player
                .unit_mut()
                .reset_attack_timer_like_cpp(wow_constants::WeaponAttackType::BaseAttack);
            (
                player.unit().mod_attack_speed_pct(),
                player
                    .unit()
                    .attack_timer(wow_constants::WeaponAttackType::BaseAttack),
                player.unit().base_attack_speed()[0],
            )
        })
        .expect("canonical player");
    assert!((hasted.0[0] - 100.0 / 130.0).abs() < 1e-6, "{:?}", hasted.0);
    assert!((hasted.0[1] - 100.0 / 130.0).abs() < 1e-6, "{:?}", hasted.0);
    assert_eq!(hasted.0[2], 1.0);
    assert_eq!(hasted.1, (hasted.2 as f32 * 100.0 / 130.0) as u32);

    session
        .apply_aura(90_992, player_guid, 30_000, 1)
        .expect("apply combat slow aura");
    // `100 / 130 * (100 + 30) / 100` for every attack.
    let slowed = session
        .canonical_player_snapshot_like_cpp(|player| player.unit().mod_attack_speed_pct())
        .expect("canonical player");
    assert!((slowed[0] - 1.0).abs() < 1e-6, "{slowed:?}");
    assert!((slowed[1] - 1.0).abs() < 1e-6, "{slowed:?}");
    assert!((slowed[2] - 1.3).abs() < 1e-6, "{slowed:?}");

    session.remove_aura(0).expect("remove melee haste aura");
    let restored = session
        .canonical_player_snapshot_like_cpp(|player| player.unit().mod_attack_speed_pct())
        .expect("canonical player");
    assert!((restored[0] - 1.3).abs() < 1e-6, "{restored:?}");
    assert!((restored[1] - 1.3).abs() < 1e-6, "{restored:?}");
    assert!((restored[2] - 1.3).abs() < 1e-6, "{restored:?}");

    session.remove_aura(1).expect("remove combat slow aura");
    assert_eq!(
        session
            .canonical_player_snapshot_like_cpp(|player| player.unit().mod_attack_speed_pct())
            .expect("canonical player"),
        [1.0, 1.0, 1.0]
    );
}

#[tokio::test]
async fn represented_shapeshift_combat_round_time_sets_form_attack_time_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 77);
    session.player_guid = Some(player_guid);
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 0, 0);
    let form_id = 8_u32;
    let spell_id = 90_993_i32;
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        represented_aura_spell_like_cpp(
            spell_id,
            wow_data::spell::aura_types::SPELL_AURA_MOD_SHAPESHIFT,
            form_id as i32,
            0,
        ),
    );
    session.set_spell_store(Arc::new(spell_store));
    session.set_spell_shapeshift_form_store(Arc::new(
        wow_data::SpellShapeshiftFormStore::from_entries([wow_data::SpellShapeshiftFormEntry {
            id: form_id,
            name: "Dire Bear Form".to_string(),
            creature_type: 0,
            flags: 0,
            attack_icon_file_id: 0,
            bonus_action_bar: 0,
            combat_round_time: 1_000,
            damage_variance: 0.0,
            mount_type_id: 0,
            creature_display_id: [0; 4],
            preset_spell_id: [0; wow_data::MAX_SHAPESHIFT_SPELLS],
        }]),
    ));

    assert_eq!(
        session.represented_shapeshift_combat_round_time_like_cpp(),
        None
    );

    session
        .apply_aura(spell_id, player_guid, 30_000, 1)
        .expect("apply shapeshift aura");
    // C++ `Player::GetShapeshiftForm` now resolves the form and
    // `InitDataForForm` writes its `CombatRoundTime` to both melee attacks.
    assert_eq!(
        session.represented_shapeshift_combat_round_time_like_cpp(),
        Some(1_000.0)
    );
    let formed = session
        .canonical_player_snapshot_like_cpp(|player| player.unit().base_attack_speed())
        .expect("canonical player");
    assert_eq!(formed[0], 1_000);
    assert_eq!(formed[1], 1_000);
    assert_eq!(formed[2], 2_000);

    session.remove_aura(0).expect("remove shapeshift aura");
    assert_eq!(
        session.represented_shapeshift_combat_round_time_like_cpp(),
        None
    );
    let restored = session
        .canonical_player_snapshot_like_cpp(|player| player.unit().base_attack_speed())
        .expect("canonical player");
    // C++ `Player::SetRegularAttackTime` only writes an attack whose equipped
    // weapon declares a delay, so this unarmed fixture keeps the form's melee
    // time while the ranged arm stays at `BASE_ATTACK_TIME`.
    assert_eq!(restored, [1_000, 1_000, 2_000]);
}
