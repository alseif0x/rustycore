// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `player.loot_item` and `player.loot_money` (#1444 H4) through the
//! production loot handlers: AzerothCore `OnPlayerLootItem`, the last step of
//! `Player::StoreLootItem` (`Player.cpp:13918`), and `LootScript::OnLootMoney`
//! (`LootHandler.cpp:273`). Each fires once, only after the durable commit.

use super::*;

#[derive(Default)]
struct LootProbe {
    seen: Mutex<Vec<String>>,
}

impl wow_module_api::PlayerModule for LootProbe {
    fn on_loot_item(
        &self,
        s: &wow_module_api::LootItemSnapshot,
        _e: &mut wow_module_api::ScopedEffects<'_>,
    ) {
        self.seen.lock().unwrap().push(format!(
            "item {} x{} from {}",
            s.item_id,
            s.count,
            s.loot_source.counter()
        ));
    }
    fn on_loot_money(
        &self,
        s: &wow_module_api::LootMoneySnapshot,
        _e: &mut wow_module_api::ScopedEffects<'_>,
    ) {
        self.seen.lock().unwrap().push(format!(
            "money {} share {} from {}",
            s.money,
            s.share,
            s.loot_source.counter()
        ));
    }
}

fn install_loot_probe(session: &mut WorldSession) -> Arc<LootProbe> {
    let probe = Arc::new(LootProbe::default());
    let mut registry = wow_module_api::ModuleRegistry::new();
    registry
        .register_module(
            wow_module_api::ModuleDescriptor::new(
                "alpha.loot",
                wow_module_api::ModuleVersion {
                    major: 1,
                    minor: 0,
                    patch: 0,
                },
                "Loot",
            )
            .unwrap(),
            wow_module_api::ModuleHooks::new().player(Arc::clone(&probe) as _),
        )
        .unwrap();
    session.install_module_hooks_like_cpp(Arc::new(registry));
    probe
}

#[tokio::test]
async fn a_stored_direct_loot_item_raises_loot_item_once() {
    let (mut first, _first_rx, _second, _second_rx, owner, _first_guid, _) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let grants = Arc::new(AtomicUsize::new(0));
    first.set_loot_item_store_test_seam_like_cpp(Arc::clone(&grants), true);
    let probe = install_loot_probe(&mut first);

    first
        .handle_loot_item(loot_item_packet(
            represented_loot_object_guid_like_cpp(owner),
            0,
        ))
        .await;
    first
        .handle_loot_item(loot_item_packet(
            represented_loot_object_guid_like_cpp(owner),
            0,
        ))
        .await;

    assert_eq!(grants.load(Ordering::SeqCst), 1);
    assert_eq!(
        *probe.seen.lock().unwrap(),
        vec![format!("item 25 x1 from {}", owner.counter())],
        "the second request finds the item taken and raises nothing"
    );
}

#[tokio::test]
async fn an_item_that_could_not_be_stored_raises_no_loot_item() {
    let (mut first, _first_rx, _second, _second_rx, owner, _first_guid, _) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let grants = Arc::new(AtomicUsize::new(0));
    first.set_loot_item_store_test_seam_like_cpp(Arc::clone(&grants), false);
    let probe = install_loot_probe(&mut first);

    first
        .handle_loot_item(loot_item_packet(
            represented_loot_object_guid_like_cpp(owner),
            0,
        ))
        .await;

    assert!(probe.seen.lock().unwrap().is_empty());
}

#[tokio::test]
async fn looted_money_raises_loot_money_once_per_request_and_only_after_the_commit() {
    let (mut first, _first_rx, _second, _second_rx, owner, _first_guid, _) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            7, false,
        ));
    let probe = install_loot_probe(&mut first);

    first.set_loot_money_persistence_test_result_like_cpp(false);
    first.handle_loot_money(loot_money_packet()).await;
    assert_eq!(first.player_gold_like_cpp(), 0);
    assert!(
        probe.seen.lock().unwrap().is_empty(),
        "a failed payment raises nothing"
    );

    first.set_loot_money_persistence_test_result_like_cpp(true);
    first.handle_loot_money(loot_money_packet()).await;
    first.handle_loot_money(loot_money_packet()).await;
    assert_eq!(first.player_gold_like_cpp(), 7);
    // AzerothCore handles a repeated request on the emptied loot too
    // (`if (loot)`, then `OnLootMoney(player, loot->gold)` with 0,
    // `LootHandler.cpp:273`): one dispatch per handled request.
    assert_eq!(
        *probe.seen.lock().unwrap(),
        vec![
            format!("money 7 share 7 from {}", owner.counter()),
            format!("money 0 share 0 from {}", owner.counter()),
        ]
    );
}
