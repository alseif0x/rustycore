use super::*;

#[tokio::test]
async fn battle_pet_grant_experience_pet_battle_uses_owner_xp_aura_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let pet_guid = ObjectGuid::create_global(HighGuid::BattlePet, 0, 0x1A7);
    let player_guid = ObjectGuid::create_player(1, 226);
    install_represented_battle_pet_stat_stores_like_cpp(&mut session);
    session.set_player_guid(Some(player_guid));
    session.set_represented_battle_pet_xp_per_level_like_cpp(23, 100);
    session.set_represented_battle_pet_xp_per_level_like_cpp(24, 100);
    session.set_represented_battle_pet_xp_per_level_like_cpp(25, 100);

    session.add_represented_battle_pet_packet_info_like_cpp(
        pet_guid,
        RepresentedBattlePetDataLikeCpp {
            species: 11,
            breed: 7,
            level: 23,
            exp: 0,
            power: 10,
            health: 50,
            max_health: 100,
            speed: 20,
            quality: 3,
            save_info: RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            ..RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
                0,
                RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            )
        },
    );
    session.send_battle_pet_journal_lock_status_like_cpp().await;
    let _ = drain_server_packet_bytes(&send_rx);
    session
        .apply_represented_battle_pet_xp_pct_aura_like_cpp(
            99_991,
            player_guid,
            &wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_BATTLE_PET_XP_PCT,
                effect_base_points: 50,
                ..Default::default()
            },
        )
        .expect("represented pet-battle XP aura");
    let _ = drain_server_packet_bytes(&send_rx);

    assert_eq!(
        session.battle_pet_grant_battle_pet_experience_with_owner_auras_like_cpp(
            pet_guid,
            200,
            RepresentedBattlePetXpSourceLikeCpp::PetBattle,
        ),
        RepresentedBattlePetGrantExperienceOutcomeLikeCpp::Changed
    );

    let pet = session
        .represented_battle_pet_like_cpp(pet_guid)
        .expect("experienced pet");
    assert_eq!(pet.level, MAX_BATTLE_PET_LEVEL_LIKE_CPP);
    assert_eq!(pet.exp, 0);
    let expected = [
        RepresentedBattlePetLevelCriteriaLikeCpp {
            species: 11,
            level: 24,
        },
        RepresentedBattlePetLevelCriteriaLikeCpp {
            species: 11,
            level: 25,
        },
    ];
    assert_eq!(
        session.represented_battle_pet_level_criteria_like_cpp(),
        &expected
    );
    assert_eq!(
        session.represented_battle_pet_active_level_criteria_like_cpp(),
        &expected
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::BattlePetUpdates]
    );
}
