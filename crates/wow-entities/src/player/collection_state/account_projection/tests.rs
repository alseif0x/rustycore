use super::*;
use crate::player_gameplay_state::PlayerAccountHeirloomDataLikeCpp;

#[test]
fn heirloom_projection_borrows_ordered_state_and_keeps_signed_filter_at_caller() {
    let mut collections = PlayerCollectionStateLikeCpp::default();
    collections.add_heirloom_like_cpp(
        20,
        PlayerAccountHeirloomDataLikeCpp {
            flags: 2,
            bonus_id: 7,
        },
    );
    collections.add_heirloom_like_cpp(
        10,
        PlayerAccountHeirloomDataLikeCpp {
            flags: 1,
            bonus_id: 5,
        },
    );
    collections.add_heirloom_like_cpp(
        u32::MAX,
        PlayerAccountHeirloomDataLikeCpp {
            flags: 3,
            bonus_id: 9,
        },
    );
    let before = collections.clone();
    let mut calls = Vec::new();
    let rows = collections.project_heirlooms(|item_id, flags, bonus_id| {
        calls.push(item_id);
        Some((i32::try_from(item_id).ok()?, flags, bonus_id))
    });
    assert_eq!(calls, vec![10, 20, u32::MAX]);
    assert_eq!(rows, vec![(10, 1, 5), (20, 2, 7)]);
    assert_eq!(collections, before);
}

#[test]
fn toy_projection_keeps_high_unsigned_ids_and_optional_rows() {
    let mut collections = PlayerCollectionStateLikeCpp::default();
    collections.add_toy_like_cpp(u32::MAX, 3);
    collections.add_toy_like_cpp(10, 0);
    let all = collections.project_toys(|item_id, flags| Some((item_id, flags)));
    assert_eq!(all, vec![(10, 0), (u32::MAX, 3)]);
    assert_eq!(
        collections.project_toys(|item_id, _| i32::try_from(item_id).ok()),
        vec![10]
    );
}

#[test]
fn mount_projection_keeps_hash_iteration_then_caller_sort_and_filter() {
    let mut collections = PlayerCollectionStateLikeCpp::default();
    collections.add_mount_like_cpp(-1, 1);
    collections.add_mount_like_cpp(20, 2);
    collections.add_mount_like_cpp(10, 3);
    let expected = collections
        .mounts_like_cpp()
        .iter()
        .map(|(id, flags)| (*id, *flags))
        .collect::<Vec<_>>();
    assert_eq!(
        collections.project_mounts(|id, flags| Some((id, flags))),
        expected
    );
    let mut positive =
        collections.project_mounts(|id, flags| Some((u32::try_from(id).ok()?, flags)));
    positive.sort_by_key(|row| row.0);
    assert_eq!(positive, vec![(10, 3), (20, 2)]);
}

#[test]
fn empty_account_projections_do_not_call_emitters() {
    let collections = PlayerCollectionStateLikeCpp::default();
    assert!(
        collections
            .project_heirlooms::<()>(|_, _, _| panic!("empty"))
            .is_empty()
    );
    assert!(
        collections
            .project_toys::<()>(|_, _| panic!("empty"))
            .is_empty()
    );
    assert!(
        collections
            .project_mounts::<()>(|_, _| panic!("empty"))
            .is_empty()
    );
}
