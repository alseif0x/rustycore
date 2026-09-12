//! #763 — the canonical Player owns its collection state and its invariants.
//!
//! C++ keeps these families in `CollectionMgr`
//! (`Entities/Player/CollectionMgr.cpp`) and performs every transition there:
//! `AddToy` (`:102`) over `UpdateAccountToys` (`:140`), `ToySetFavorite`
//! (`:145`), `ToyClearFanfare` (`:157`), `AddHeirloom` (`:237`) over
//! `UpdateAccountHeirlooms` (`:217`), `UpgradeHeirloom` (`:243`),
//! `CheckHeirloomUpgrades` (`:278`), `AddMount` (`:360`), `MountSetFavorite`
//! (`:395`), `SaveAccountItemAppearances` (`:516`), `AddItemAppearance`
//! (`:732`), `AddTemporaryAppearance` (`:768`), `RemoveTemporaryAppearance`
//! (`:777`) and `SetAppearanceIsFavorite` (`:828`).

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::{
    PlayerAccountHeirloomDataLikeCpp, PlayerCollectionStateLikeCpp,
    PlayerFavoriteAppearanceStateLikeCpp,
};

fn item_guid(db_id: i64) -> wow_core::ObjectGuid {
    wow_core::ObjectGuid::create_item(1, db_id)
}

#[test]
fn a_fresh_collection_state_is_empty_like_cpp() {
    let state = PlayerCollectionStateLikeCpp::default();

    assert!(state.mounts_like_cpp().is_empty());
    assert!(state.heirlooms_like_cpp().is_empty());
    assert!(state.toys_like_cpp().is_empty());
    assert!(state.item_appearances_like_cpp().is_empty());
    assert!(state.item_appearance_blocks_like_cpp().is_empty());
    assert!(state.temporary_item_appearances_like_cpp().is_empty());
    assert!(state.favorite_item_appearances_like_cpp().is_empty());
    assert!(state.transmog_illusions_like_cpp().is_empty());
}

#[test]
fn a_collected_toy_is_not_re_added_like_cpp_update_account_toys() {
    let mut state = PlayerCollectionStateLikeCpp::default();

    assert!(state.add_toy_like_cpp(100, 0x01));
    assert!(!state.add_toy_like_cpp(100, 0x02));
    assert_eq!(state.toys_like_cpp().get(&100), Some(&0x01));
}

#[test]
fn toy_flag_updates_set_and_clear_only_the_named_bits_like_cpp() {
    let mut state = PlayerCollectionStateLikeCpp::default();
    state.add_toy_like_cpp(100, 0b0000_0101);

    assert!(state.update_toy_flags_like_cpp(100, 0b0000_1000, 0b0000_0001));
    assert_eq!(state.toys_like_cpp().get(&100), Some(&0b0000_1100));
    assert!(!state.update_toy_flags_like_cpp(101, 0b0000_0001, 0));
}

#[test]
fn removing_a_toy_reports_whether_it_was_collected_like_cpp() {
    let mut state = PlayerCollectionStateLikeCpp::default();
    state.add_toy_like_cpp(100, 0);

    assert!(state.remove_toy_like_cpp(100));
    assert!(!state.remove_toy_like_cpp(100));
    assert!(state.toys_like_cpp().is_empty());
}

#[test]
fn a_collected_heirloom_is_not_replaced_like_cpp_update_account_heirlooms() {
    let mut state = PlayerCollectionStateLikeCpp::default();

    assert!(state.add_heirloom_like_cpp(
        200,
        PlayerAccountHeirloomDataLikeCpp {
            flags: 1,
            bonus_id: 7,
        },
    ));
    assert!(!state.add_heirloom_like_cpp(
        200,
        PlayerAccountHeirloomDataLikeCpp {
            flags: 9,
            bonus_id: 9,
        },
    ));
    assert_eq!(
        state.heirlooms_like_cpp().get(&200).map(|d| d.flags),
        Some(1)
    );
    assert_eq!(
        state.heirlooms_like_cpp().get(&200).map(|d| d.bonus_id),
        Some(7)
    );
}

#[test]
fn upgrading_an_absent_heirloom_writes_nothing_like_cpp() {
    let mut state = PlayerCollectionStateLikeCpp::default();

    assert!(!state.update_heirloom_like_cpp(200, 3, 4));
    assert!(state.heirlooms_like_cpp().is_empty());
}

#[test]
fn a_superseded_heirloom_is_replaced_by_a_successor_without_flags_like_cpp() {
    let mut state = PlayerCollectionStateLikeCpp::default();
    state.add_heirloom_like_cpp(
        200,
        PlayerAccountHeirloomDataLikeCpp {
            flags: 5,
            bonus_id: 6,
        },
    );

    state.replace_heirloom_like_cpp(200, 201);

    assert!(!state.heirlooms_like_cpp().contains_key(&200));
    assert_eq!(
        state.heirlooms_like_cpp().get(&201),
        Some(&PlayerAccountHeirloomDataLikeCpp {
            flags: 0,
            bonus_id: 0,
        })
    );
}

#[test]
fn a_collected_mount_is_not_overwritten_like_cpp_mounts_insert() {
    let mut state = PlayerCollectionStateLikeCpp::default();

    assert!(state.add_mount_like_cpp(300, 0x01));
    assert!(!state.add_mount_like_cpp(300, 0x02));
    assert_eq!(state.mounts_like_cpp().get(&300), Some(&0x01));
}

#[test]
fn mount_flag_updates_need_a_collected_mount_like_cpp_mount_set_favorite() {
    let mut state = PlayerCollectionStateLikeCpp::default();
    state.add_mount_like_cpp(300, 0b0000_0010);

    assert!(state.update_mount_flags_like_cpp(300, 0b0000_0001, 0b0000_0010));
    assert_eq!(state.mounts_like_cpp().get(&300), Some(&0b0000_0001));
    assert!(!state.update_mount_flags_like_cpp(301, 0b0000_0001, 0));
}

#[test]
fn a_permanent_appearance_drops_the_temporary_providers_of_the_same_appearance_like_cpp() {
    let mut state = PlayerCollectionStateLikeCpp::default();
    assert!(state.add_temporary_item_appearance_like_cpp(400, item_guid(1)));
    assert!(state.has_temporary_item_appearance_like_cpp(400));

    state.add_item_appearance_like_cpp(400);

    assert!(state.item_appearances_like_cpp().contains(&400));
    assert!(!state.has_temporary_item_appearance_like_cpp(400));
    assert!(state.temporary_item_appearances_like_cpp().is_empty());
}

#[test]
fn only_the_first_temporary_provider_reports_a_new_conditional_transmog_like_cpp() {
    let mut state = PlayerCollectionStateLikeCpp::default();

    assert!(state.add_temporary_item_appearance_like_cpp(400, item_guid(1)));
    assert!(!state.add_temporary_item_appearance_like_cpp(400, item_guid(2)));
    assert_eq!(
        state
            .temporary_item_appearances_like_cpp()
            .get(&400)
            .map(HashSet::len),
        Some(2)
    );
}

#[test]
fn a_temporary_appearance_is_erased_only_with_its_last_provider_like_cpp() {
    let mut state = PlayerCollectionStateLikeCpp::default();
    state.add_temporary_item_appearance_like_cpp(400, item_guid(1));
    state.add_temporary_item_appearance_like_cpp(400, item_guid(2));

    assert!(!state.remove_temporary_item_appearance_like_cpp(400, item_guid(1)));
    assert!(state.has_temporary_item_appearance_like_cpp(400));
    assert!(state.remove_temporary_item_appearance_like_cpp(400, item_guid(2)));
    assert!(!state.has_temporary_item_appearance_like_cpp(400));

    assert!(!state.remove_temporary_item_appearance_like_cpp(400, item_guid(2)));
    assert!(!state.remove_temporary_item_appearance_like_cpp(401, item_guid(1)));
}

#[test]
fn an_unknown_provider_never_erases_a_temporary_appearance_like_cpp() {
    let mut state = PlayerCollectionStateLikeCpp::default();
    state.add_temporary_item_appearance_like_cpp(400, item_guid(1));

    assert!(!state.remove_temporary_item_appearance_like_cpp(400, item_guid(2)));
    assert!(state.has_temporary_item_appearance_like_cpp(400));
}

#[test]
fn saving_favorite_appearances_settles_new_and_removed_entries_like_cpp() {
    let mut state = PlayerCollectionStateLikeCpp::default();
    state.install_appearance_collection_like_cpp(
        HashSet::from([400, 401, 402]),
        vec![0b111],
        HashMap::from([
            (400, PlayerFavoriteAppearanceStateLikeCpp::New),
            (401, PlayerFavoriteAppearanceStateLikeCpp::Removed),
            (402, PlayerFavoriteAppearanceStateLikeCpp::Unchanged),
        ]),
    );

    let (inserts, deletes) = state.settle_favorite_item_appearance_saves_like_cpp();

    assert_eq!(inserts, vec![400]);
    assert_eq!(deletes, vec![401]);
    assert_eq!(
        state.favorite_item_appearances_like_cpp().get(&400),
        Some(&PlayerFavoriteAppearanceStateLikeCpp::Unchanged)
    );
    assert!(
        !state
            .favorite_item_appearances_like_cpp()
            .contains_key(&401)
    );
    assert_eq!(
        state.favorite_item_appearances_like_cpp().get(&402),
        Some(&PlayerFavoriteAppearanceStateLikeCpp::Unchanged)
    );
}

#[test]
fn a_settled_save_is_idempotent_until_a_new_favorite_transition_like_cpp() {
    let mut state = PlayerCollectionStateLikeCpp::default();
    state.install_appearance_collection_like_cpp(
        HashSet::from([400]),
        vec![0b1],
        HashMap::from([(400, PlayerFavoriteAppearanceStateLikeCpp::New)]),
    );
    let _ = state.settle_favorite_item_appearance_saves_like_cpp();

    let (inserts, deletes) = state.settle_favorite_item_appearance_saves_like_cpp();

    assert!(inserts.is_empty());
    assert!(deletes.is_empty());
}

#[test]
fn installing_an_appearance_collection_replaces_the_three_authoritative_parts_like_cpp() {
    let mut state = PlayerCollectionStateLikeCpp::default();
    state.add_item_appearance_like_cpp(399);

    state.install_appearance_collection_like_cpp(
        HashSet::from([400]),
        vec![0b1],
        HashMap::from([(400, PlayerFavoriteAppearanceStateLikeCpp::Unchanged)]),
    );

    assert_eq!(state.item_appearances_like_cpp(), &HashSet::from([400]));
    assert_eq!(state.item_appearance_blocks_like_cpp(), [0b1]);
    assert_eq!(state.favorite_item_appearances_like_cpp().len(), 1);
}

#[test]
fn the_loaded_account_parts_build_the_collection_the_owner_receives_like_cpp() {
    let state = PlayerCollectionStateLikeCpp::from_loaded_account_parts_like_cpp(
        HashMap::from([(300, 1)]),
        BTreeMap::from([(
            200,
            PlayerAccountHeirloomDataLikeCpp {
                flags: 2,
                bonus_id: 3,
            },
        )]),
        BTreeMap::from([(100, 4)]),
        HashSet::from([400]),
        vec![0b1],
        HashMap::from([(401, HashSet::from([item_guid(1)]))]),
        HashMap::from([(400, PlayerFavoriteAppearanceStateLikeCpp::Unchanged)]),
        HashSet::from([500]),
    );

    assert_eq!(state.mounts_like_cpp().get(&300), Some(&1));
    assert_eq!(state.toys_like_cpp().get(&100), Some(&4));
    assert!(state.heirlooms_like_cpp().contains_key(&200));
    assert!(state.has_temporary_item_appearance_like_cpp(401));
    assert!(state.transmog_illusions_like_cpp().contains(&500));
    assert_eq!(state.mounts_snapshot_like_cpp(), *state.mounts_like_cpp());
    assert_eq!(
        state.item_appearance_blocks_snapshot_like_cpp(),
        state.item_appearance_blocks_like_cpp()
    );
}
