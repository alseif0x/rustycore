//! Skill scenarios for [`super`].
//!
//! Split out of character_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn level_up_deltas_use_cpp_class_race_stats_and_base_mp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 1, 0);
    session.set_player_stats(Arc::new(PlayerStatsStore::from_entries([
        (
            (1, 5, 1),
            PlayerLevelStats {
                strength: 10,
                agility: 11,
                stamina: 12,
                intellect: 13,
                spirit: 14,
                base_mana: 155,
            },
        ),
        (
            (1, 5, 2),
            PlayerLevelStats {
                strength: 12,
                agility: 11,
                stamina: 15,
                intellect: 17,
                spirit: 19,
                base_mana: 170,
            },
        ),
    ])));

    assert_eq!(
        session.level_up_stat_deltas_like_cpp(2),
        Some((15, [2, 0, 3, 4, 5]))
    );
    assert_eq!(session.level_up_stat_deltas_like_cpp(3), None);
}
#[test]
fn level_up_stat_update_refills_health_and_mana_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    let player_guid = ObjectGuid::create_player(1, 76);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    set_priest_level80_stats(&mut session, 1000, 40);
    attach_stat_update_player_with_mana_and_health(&mut session, player_guid, 777, 1320, 3, 10);

    session.send_level_up_stat_update_like_cpp();

    assert_eq!(
        session.canonical_player_health_snapshot_like_cpp(),
        Some((10, 10))
    );
    assert_eq!(
        session.canonical_player_power_snapshot_like_cpp(PowerType::Mana),
        Some((1320, 1320))
    );
    assert_eq!(session.player_health_like_cpp(), 10);
}
#[tokio::test]
async fn opening_cinematic_requires_zero_xp_and_prefers_class_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    session.set_loaded_player_identity_like_cpp(571, 1, 8, 1, 0);
    session.set_player_xp_like_cpp(1);
    session.set_chr_classes_store(Arc::new(ChrClassesStore::from_entries([chr_class_entry(
        8, 111,
    )])));
    session.set_chr_races_store(Arc::new(ChrRacesStore::from_entries([chr_race_entry(
        1, 222,
    )])));

    session
        .handle_opening_cinematic(WorldPacket::new_empty())
        .await;
    assert!(send_rx.try_recv().is_err());

    session.set_player_xp_like_cpp(0);
    session
        .handle_opening_cinematic(WorldPacket::new_empty())
        .await;
    assert_eq!(send_rx.try_recv().unwrap(), expected_trigger_cinematic(111));

    let (mut fallback, fallback_rx) = make_session_with_send_capacity(4);
    fallback.set_loaded_player_identity_like_cpp(571, 1, 8, 1, 0);
    fallback.set_player_xp_like_cpp(0);
    fallback.set_chr_classes_store(Arc::new(ChrClassesStore::from_entries([chr_class_entry(
        8, 0,
    )])));
    fallback.set_chr_races_store(Arc::new(ChrRacesStore::from_entries([chr_race_entry(
        1, 222,
    )])));

    fallback
        .handle_opening_cinematic(WorldPacket::new_empty())
        .await;
    assert_eq!(
        fallback_rx.try_recv().unwrap(),
        expected_trigger_cinematic(222)
    );
}
#[test]
fn vendor_required_reputation_fails_closed_until_reputation_mgr_exists() {
    assert_eq!(
        vendor_buy_required_reputation_block_result(None, None, -1),
        None
    );
    assert_eq!(
        vendor_buy_required_reputation_block_result(Some(72), Some(5), -1),
        Some(BuyResult::ReputationRequire)
    );
    assert_eq!(
        vendor_buy_required_reputation_block_result(Some(72), Some(5), 5),
        None
    );
}
