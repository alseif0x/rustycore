//! Preserved canonical loot lifecycle scenarios.
use super::money_support::*;

#[tokio::test]
async fn durable_old_generation_payout_does_not_touch_replacement_loot_like_cpp() {
    let (mut first, first_rx, _second, _second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            7, false,
        ));
    let _ = drain_server_opcodes_like_cpp(&first_rx);
    let authority = loot_recovery_authority_for_test(&mut first, owner)
        .unwrap();
    let old_generation = active_money_generation_for_test(&first, owner)
        .expect("opened loot generation");
    let mut replacement = authoritative_test_loot_like_cpp(17, false);
    replacement.loot_guid = represented_loot_object_guid_like_cpp(owner);
    replacement.allowed_looters = vec![first_guid, second_guid];
    authority.replace_like_cpp(Some(replacement), HashMap::new());
    authority.add_viewer_like_cpp(first_guid).unwrap();

    apply_money_command_for_test(&mut first, LootMoneyApplication::new(
            first_guid, owner, represented_loot_object_guid_like_cpp(owner),
            3, Arc::new(AtomicU64::new(3)), Default::default(), true, authority.clone(),
            old_generation, Arc::new(AtomicBool::new(true)), Arc::new(AtomicBool::new(true)),
            Arc::new(AtomicBool::new(false)), Arc::new(AtomicBool::new(false)),
        ))
        .await;

    assert_eq!(player_gold_for_test(&first), 3);
    let snapshot = authority.snapshot_for_player_like_cpp(first_guid).unwrap();
    assert_eq!(snapshot.loot.coins, 17);
    assert!(snapshot.loot.players_looting.contains(&first_guid));
    let opcodes = drain_server_opcodes_like_cpp(&first_rx);
    assert!(!opcodes.contains(&(wow_constants::ServerOpcodes::CoinRemoved as u16)));
    assert_eq!(
        opcodes
            .iter()
            .filter(|opcode| { **opcode == wow_constants::ServerOpcodes::LootMoneyNotify as u16 })
            .count(),
        1
    );
}
