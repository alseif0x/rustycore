// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `item.equipped` (#1444 H4) through the production equip paths: AzerothCore
//! `Player::EquipItem` -> `OnPlayerEquip` (`PlayerStorage.cpp:2940`), raised
//! once per item that reaches an equipment position, after the move committed.

use super::*;

#[derive(Default)]
struct EquipProbe {
    seen: std::sync::Mutex<Vec<(ObjectGuid, u32, u8, u8)>>,
}

impl wow_module_api::PlayerModule for EquipProbe {
    fn on_equip(
        &self,
        snapshot: &wow_module_api::ItemEquippedSnapshot,
        _effects: &mut wow_module_api::ScopedEffects<'_>,
    ) {
        self.seen.lock().unwrap().push((
            snapshot.item_guid,
            snapshot.item_entry,
            snapshot.bag,
            snapshot.slot,
        ));
    }
}

fn install_equip_probe(session: &mut WorldSession) -> Arc<EquipProbe> {
    let probe = Arc::new(EquipProbe::default());
    let mut registry = wow_module_api::ModuleRegistry::new();
    registry
        .register_module(
            wow_module_api::ModuleDescriptor::new(
                "alpha.equip",
                wow_module_api::ModuleVersion {
                    major: 1,
                    minor: 0,
                    patch: 0,
                },
                "Equip",
            )
            .unwrap(),
            wow_module_api::ModuleHooks::new().player(Arc::clone(&probe) as _),
        )
        .unwrap();
    session.install_module_hooks_like_cpp(Arc::new(registry));
    probe
}

fn equip_session(
    outcome: PersistenceOutcomeLikeCpp,
) -> (WorldSession, flume::Receiver<Vec<u8>>, Arc<EquipProbe>) {
    let (mut session, send_rx) = make_session_with_send_capacity(64);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 44_001)));
    install_equippable_item_fixture(&mut session, 44_001, InventoryType::Weapon, Some(3));
    let (port, _requests) = PlayerInventoryPersistencePortFixtureLikeCpp::new_like_cpp(outcome);
    session.set_player_inventory_persistence_port_like_cpp(port);
    let probe = install_equip_probe(&mut session);
    (session, send_rx, probe)
}

#[tokio::test]
async fn equipping_into_an_empty_slot_raises_item_equipped_once_after_the_commit() {
    let (mut session, _send_rx, probe) =
        equip_session(PersistenceOutcomeLikeCpp::Applied { rows: 1 });
    let item = insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        44_001,
        44_001,
        InventoryType::Weapon,
    );

    session
        .handle_auto_equip_item_slot(AutoEquipItemSlot {
            inv_update: InvUpdate {
                items: vec![(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)],
            },
            item,
            item_dst_slot: EQUIPMENT_SLOT_MAINHAND,
        })
        .await;

    assert_eq!(
        *probe.seen.lock().unwrap(),
        vec![(item, 44_001, INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND)]
    );
}

#[tokio::test]
async fn a_failed_or_unknown_equip_raises_no_item_equipped() {
    for outcome in [
        PersistenceOutcomeLikeCpp::Failed {
            reason: "fixture".to_owned(),
        },
        PersistenceOutcomeLikeCpp::Unknown {
            reason: "fixture".to_owned(),
        },
    ] {
        let (mut session, _send_rx, probe) = equip_session(outcome);
        let item = insert_equippable_test_item(
            &mut session,
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            44_001,
            44_002,
            InventoryType::Weapon,
        );
        session
            .handle_auto_equip_item_slot(AutoEquipItemSlot {
                inv_update: InvUpdate {
                    items: vec![(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)],
                },
                item,
                item_dst_slot: EQUIPMENT_SLOT_MAINHAND,
            })
            .await;
        assert!(probe.seen.lock().unwrap().is_empty());
        assert!(
            session
                .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND)
                .is_none()
        );
    }
}

/// An occupied slot is a real swap: the new item is equipped and raises the
/// hook; the old one goes to the backpack, which is no equipment position.
#[tokio::test]
async fn a_swap_raises_item_equipped_only_for_the_item_that_reaches_the_slot() {
    let (mut session, _send_rx, probe) =
        equip_session(PersistenceOutcomeLikeCpp::Applied { rows: 1 });
    let worn = insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        EQUIPMENT_SLOT_MAINHAND,
        44_001,
        44_003,
        InventoryType::Weapon,
    );
    let new = insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        44_001,
        44_004,
        InventoryType::Weapon,
    );

    session
        .handle_auto_equip_item_slot(AutoEquipItemSlot {
            inv_update: InvUpdate {
                items: vec![(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)],
            },
            item: new,
            item_dst_slot: EQUIPMENT_SLOT_MAINHAND,
        })
        .await;

    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND)
            .map(|item| item.guid),
        Some(new),
        "the swap committed"
    );
    assert_eq!(
        *probe.seen.lock().unwrap(),
        vec![(new, 44_001, INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND)]
    );
    let _ = worn;
}
