//! Original creature personal-pool generation and lifetime cases.

use super::creature_pool_setup::*;
use super::recovery_support::*;
use std::collections::HashMap;
use wow_world::test_fixtures::loot::{
    bind_loot_view_for_test, creature_loot_revision_for_test, ensure_creature_kill_loot_for_test,
    has_cached_loot_generation_for_test, install_creature_kill_loot_for_test,
    loot_response_for_test, mutate_loot_creature_for_test, personal_loot_money_entry_for_test,
    release_loot_owner_for_test,
};

#[tokio::test]
async fn overworld_creature_builds_independent_personal_loot_per_connected_tapper_like_cpp() {
    let mut fixture = overworld_personal_loot_test_fixture_like_cpp();

    ensure_creature_kill_loot_for_test(&mut fixture.session, fixture.owner_guid).await;

    let authority = loot_recovery_authority_for_test(&mut fixture.session, fixture.owner_guid)
        .expect("the dead creature keeps its object-owned loot authority");
    let (first_normal_slot, second_normal_slot) =
        assert_overworld_personal_loot_generation_like_cpp(&authority, &fixture);
    assert_overworld_personal_loot_claims_are_independent_like_cpp(
        &authority,
        fixture.first_tapper,
        first_normal_slot,
        fixture.second_tapper,
        second_normal_slot,
    )
    .await;
}

#[tokio::test]
async fn cmsg_loot_unit_never_regenerates_after_creature_clear_loot_like_cpp() {
    let mut fixture = overworld_personal_loot_test_fixture_like_cpp();
    ensure_creature_kill_loot_for_test(&mut fixture.session, fixture.owner_guid).await;
    let authority =
        loot_recovery_authority_for_test(&mut fixture.session, fixture.owner_guid).unwrap();
    mutate_loot_creature_for_test(&mut fixture.session, fixture.owner_guid, |creature| {
        creature.creature.clear_loot_like_cpp();
    });
    let retired_generation = authority.generation_like_cpp();

    assert!(
        loot_response_for_test(
            &mut fixture.session,
            fixture.owner_guid,
            fixture.first_tapper,
            false,
        )
        .await
        .is_none()
    );
    assert!(authority.is_retired_like_cpp());
    assert_eq!(authority.generation_like_cpp(), retired_generation);
    assert!(
        authority
            .snapshot_for_player_like_cpp(fixture.first_tapper)
            .is_none()
    );
}

#[test]
fn stale_kill_generator_cannot_install_after_creature_lifecycle_aba_like_cpp() {
    let mut fixture = overworld_personal_loot_test_fixture_like_cpp();
    let authority =
        loot_recovery_authority_for_test(&mut fixture.session, fixture.owner_guid).unwrap();
    let expected_generation = authority.generation_like_cpp();
    let expected_revision =
        creature_loot_revision_for_test(&mut fixture.session, fixture.owner_guid).unwrap();
    let mut stale_pool = authoritative_test_loot_like_cpp(0, true);
    stale_pool.loot_guid = represented_loot_object_guid_like_cpp(fixture.owner_guid);
    stale_pool.allowed_looters = vec![fixture.first_tapper];
    stale_pool.items[0].allowed_looters = vec![fixture.first_tapper];

    mutate_loot_creature_for_test(&mut fixture.session, fixture.owner_guid, |creature| {
        creature.creature.clear_loot_like_cpp();
        creature
            .creature
            .set_death_state_runtime(wow_constants::DeathState::JustRespawned, 0);
        creature
            .creature
            .set_death_state_runtime(wow_constants::DeathState::JustDied, 0);
    });

    assert!(!install_creature_kill_loot_for_test(
        &mut fixture.session,
        fixture.owner_guid,
        &authority,
        expected_generation,
        expected_revision,
        None,
        HashMap::from([(fixture.first_tapper, stale_pool)]),
    ));
    assert!(authority.is_retired_like_cpp());
    assert!(
        authority
            .snapshot_for_player_like_cpp(fixture.first_tapper)
            .is_none()
    );
}

#[tokio::test]
async fn authoritative_partial_personal_creature_release_drops_cache_and_reopen_rehydrates_like_cpp()
 {
    let mut fixture = overworld_personal_loot_test_fixture_like_cpp();
    ensure_creature_kill_loot_for_test(&mut fixture.session, fixture.owner_guid).await;
    let authority =
        loot_recovery_authority_for_test(&mut fixture.session, fixture.owner_guid).unwrap();
    let before_release = authority
        .snapshot_for_player_like_cpp(fixture.first_tapper)
        .unwrap();
    let slot = before_release
        .loot
        .items
        .iter()
        .find(|item| item.item_id == fixture.normal_item_id)
        .unwrap()
        .loot_list_id;
    assert!(reconcile_loot_cache_for_test(
        &mut fixture.session,
        fixture.owner_guid,
        fixture.first_tapper,
    ));
    let opened = authority
        .add_viewer_like_cpp(fixture.first_tapper)
        .expect("the authoritative personal pool opens");
    set_active_loot_guid_for_test(&mut fixture.session, fixture.owner_guid);
    bind_loot_view_for_test(
        &mut fixture.session,
        fixture.owner_guid,
        opened.generation,
        &authority,
    );

    assert!(
        release_loot_owner_for_test(
            &mut fixture.session,
            fixture.owner_guid,
            fixture.first_tapper
        )
        .await
    );
    assert!(!has_loot_for_test(&fixture.session, fixture.owner_guid));
    assert!(!has_cached_loot_generation_for_test(
        &fixture.session,
        fixture.owner_guid
    ));
    assert!(
        !personal_loot_money_entry_for_test(
            &fixture.session,
            fixture.owner_guid,
            fixture.first_tapper
        )
        .is_some()
    );
    let after_release = authority
        .snapshot_for_player_like_cpp(fixture.first_tapper)
        .unwrap();
    assert_eq!(after_release.generation, before_release.generation);
    assert_eq!(after_release.scope, before_release.scope);
    assert_eq!(after_release.loot.coins, before_release.loot.coins);
    assert_eq!(after_release.loot.items, before_release.loot.items);
    assert!(after_release.loot.players_looting.is_empty());
    assert!(
        after_release.loot.looted_by_player,
        "C++ keeps the per-Loot was-opened state after closing the viewer"
    );

    let response = loot_response_for_test(
        &mut fixture.session,
        fixture.owner_guid,
        fixture.first_tapper,
        false,
    )
    .await
    .expect("the creature authority rehydrates a personal view");
    assert_eq!(response.coins, 7);
    assert!(
        personal_loot_marker_for_test(&fixture.session, fixture.owner_guid, fixture.first_tapper).0
    );
    assert_eq!(
        personal_loot_money_entry_for_test(
            &fixture.session,
            fixture.owner_guid,
            fixture.first_tapper
        ),
        Some(&7)
    );
    authority
        .reserve_item_like_cpp(fixture.first_tapper, slot)
        .await
        .unwrap()
        .commit_like_cpp()
        .unwrap();
    assert!(
        authority
            .reserve_item_like_cpp(fixture.first_tapper, slot)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn dungeon_trash_builds_one_personal_pool_for_selected_group_looter_like_cpp() {
    let mut fixture = overworld_personal_loot_test_fixture_like_cpp();
    fixture
        .session
        .set_map_store(Arc::new(wow_data::MapStore::from_entries([
            wow_data::MapEntry {
                id: 0,
                instance_type: wow_data::map::MAP_INSTANCE,
                expansion_id: 0,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: 0,
                flags2: 0,
            },
        ])));
    let groups = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(fixture.first_tapper);
    group.add_member(fixture.second_tapper);
    group.looter_guid = fixture.second_tapper;
    let group_guid = group.group_guid;
    groups.register_group_like_cpp(group_guid, group);
    set_group_guid_for_test_like_cpp(&mut fixture.session, Some(group_guid));
    fixture
        .session
        .set_group_registry(Arc::clone(&groups), Arc::new(PendingInvites::default()));

    ensure_creature_kill_loot_for_test(&mut fixture.session, fixture.owner_guid).await;

    let authority =
        loot_recovery_authority_for_test(&mut fixture.session, fixture.owner_guid).unwrap();
    let personal = authority.personal_snapshots_like_cpp();
    assert_eq!(personal.len(), 1);
    assert!(!personal.contains_key(&fixture.first_tapper));
    let selected = &personal[&fixture.second_tapper].loot;
    assert_eq!(selected.dungeon_encounter_id, 0);
    assert_eq!(selected.allowed_looters, vec![fixture.second_tapper]);
    assert!(
        selected
            .items
            .iter()
            .all(|entry| { entry.allowed_looters == vec![fixture.second_tapper] })
    );
    assert_eq!(
        groups.get(&group_guid).unwrap().looter_guid_like_cpp(),
        fixture.first_tapper,
        "non-empty dungeon trash advances the round-robin group looter"
    );
}
