//! Failed or incoherent production loads cannot authorize the obtain scan.

use super::*;
use wow_persistence::{
    PlayerBagInventoryLoadRowLikeCpp, PlayerEquipmentInventoryLoadRowLikeCpp,
    PlayerInventoryItemLoadRowLikeCpp,
};

#[tokio::test]
async fn carried_obtain_readiness_requires_both_queries_and_coherent_rows() {
    let row = |id| PlayerInventoryItemLoadRowLikeCpp {
        item_entry: 2121,
        item_db_guid: id,
        count: 1,
        durability: 40,
        context: 0,
        flags: 0,
        played_time: 0,
        enchantments: String::new(),
        random_properties_id: 0,
        random_properties_seed: 0,
        gems: std::array::from_fn(|_| (0, String::new(), 0)),
        paid_money: None,
        paid_extended_cost: None,
        expiration: 0,
        spell_charges: String::new(),
    };
    let equipment = |rows| {
        PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
            PlayerLoginAuxiliaryLoadedLikeCpp::EquipmentInventory(rows),
        )
    };
    let bags = |rows| {
        PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
            PlayerLoginAuxiliaryLoadedLikeCpp::BagInventory(rows),
        )
    };
    let failed = || PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed {
        reason: "controlled inventory query failure".into(),
    };
    let top = || PlayerEquipmentInventoryLoadRowLikeCpp {
        slot: wow_entities::INVENTORY_SLOT_ITEM_START,
        item: row(9301),
    };
    let mut skipped = top();
    skipped.item.item_entry = 0;
    let mut conflicting = top();
    conflicting.item.item_db_guid = 9302;
    for (equipment_result, bag_result, ready) in [
        (failed(), bags(vec![]), false),
        (equipment(vec![top()]), failed(), false),
        (equipment(vec![skipped]), bags(vec![]), false),
        (equipment(vec![top(), conflicting]), bags(vec![]), false),
        (
            equipment(vec![top()]),
            bags(vec![PlayerBagInventoryLoadRowLikeCpp {
                bag_slot: wow_entities::INVENTORY_SLOT_BAG_START,
                inner_slot: 0,
                item: row(9303),
            }]),
            false,
        ),
        (equipment(vec![]), bags(vec![]), true),
    ] {
        let (mut session, _, _) = make_session();
        let owner = install_canonical_player_owner_for_test(&mut session, 571, 0);
        session
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player
                    .inventory_runtime_mut_like_cpp()
                    .set_carried_inventory_loaded_like_cpp(true);
            })
            .unwrap();
        let port: Arc<dyn PlayerLifecyclePortLikeCpp> = AuxiliaryLoadPortLikeCpp::new([
            equipment_result,
            bag_result,
            PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                PlayerLoginAuxiliaryLoadedLikeCpp::EquipmentSets(vec![]),
            ),
            PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                PlayerLoginAuxiliaryLoadedLikeCpp::TransmogOutfits(vec![]),
            ),
        ]);
        wow_world_character::load_inventory_for_login_like_cpp(&mut session, &port, owner, 1)
            .await
            .expect("the existing login snapshot remains available");
        session
            .core
            .with_owned_player_like_cpp(|player| {
                assert_eq!(
                    player
                        .inventory_runtime_like_cpp()
                        .carried_inventory_loaded_like_cpp(),
                    ready
                );
                assert_eq!(
                    player
                        .inventory_runtime_like_cpp()
                        .equipment_inventory_authority_complete_like_cpp(),
                    ready
                );
                assert_eq!(
                    player.cast_all_obtain_backpack_end_like_cpp().is_some(),
                    ready
                );
            })
            .unwrap();
    }
}
