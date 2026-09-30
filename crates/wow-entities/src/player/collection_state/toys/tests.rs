use super::PlayerCollectionStateLikeCpp;

#[test]
fn add_account_toy_inserts_once_like_cpp() {
    let mut collections = PlayerCollectionStateLikeCpp::default();

    assert!(collections.add_toy_like_cpp(
        30_000,
        PlayerCollectionStateLikeCpp::toy_flags(false, false),
    ));
    assert!(!collections.add_toy_like_cpp(
        30_000,
        PlayerCollectionStateLikeCpp::toy_flags(true, true),
    ));

    assert_eq!(
        collections.project_toy_status(|item_id, is_favorite, has_fanfare| {
            Some((item_id, is_favorite, has_fanfare))
        }),
        vec![(30_000, false, false)]
    );
}

#[test]
fn toy_clear_fanfare_clears_known_toy_only_like_cpp() {
    let mut collections = PlayerCollectionStateLikeCpp::default();
    collections.replace_toys_like_cpp(PlayerCollectionStateLikeCpp::prepare_toys([
        (30_000, true, true),
        (30_001, false, true),
    ]));

    assert!(collections.clear_toy_fanfare(30_000));
    assert!(!collections.clear_toy_fanfare(40_000));

    assert_eq!(
        collections.project_toy_status(|item_id, is_favorite, has_fanfare| {
            Some((item_id, is_favorite, has_fanfare))
        }),
        vec![(30_000, true, false), (30_001, false, true)]
    );
}

#[test]
fn toy_set_favorite_toggles_known_toy_only_like_cpp() {
    let mut collections = PlayerCollectionStateLikeCpp::default();
    collections.replace_toys_like_cpp(PlayerCollectionStateLikeCpp::prepare_toys([
        (30_000, false, true),
    ]));

    assert!(collections.set_toy_favorite(30_000, true));
    assert_eq!(
        collections.project_toy_status(|item_id, is_favorite, has_fanfare| {
            Some((item_id, is_favorite, has_fanfare))
        }),
        vec![(30_000, true, true)]
    );

    assert!(collections.set_toy_favorite(30_000, false));
    assert!(!collections.set_toy_favorite(40_000, true));
    assert_eq!(
        collections.project_toy_status(|item_id, is_favorite, has_fanfare| {
            Some((item_id, is_favorite, has_fanfare))
        }),
        vec![(30_000, false, true)]
    );
}

#[test]
fn unknown_toy_transitions_do_not_insert_rows() {
    let mut collections = PlayerCollectionStateLikeCpp::default();
    assert!(!collections.clear_toy_fanfare(40_000));
    assert!(!collections.set_toy_favorite(40_000, true));
    assert!(!collections.set_toy_favorite(40_000, false));
    assert!(collections.toys_like_cpp().is_empty());
}

#[test]
fn known_toy_transitions_succeed_idempotently_and_preserve_other_bits() {
    let mut collections = PlayerCollectionStateLikeCpp::default();
    collections.add_toy_like_cpp(30_000, 0x83);
    assert!(collections.set_toy_favorite(30_000, true));
    assert_eq!(collections.toys_like_cpp()[&30_000], 0x83);
    assert!(collections.clear_toy_fanfare(30_000));
    assert!(collections.clear_toy_fanfare(30_000));
    assert_eq!(collections.toys_like_cpp()[&30_000], 0x81);
    assert!(collections.set_toy_favorite(30_000, false));
    assert!(collections.set_toy_favorite(30_000, false));
    assert_eq!(collections.toys_like_cpp()[&30_000], 0x80);
}

#[test]
fn toy_status_projection_keeps_row_order_optional_emission_and_unrelated_bits() {
    let mut collections = PlayerCollectionStateLikeCpp::default();
    collections.add_toy_like_cpp(30, 0x82);
    collections.add_toy_like_cpp(10, 0x80);
    collections.add_toy_like_cpp(20, 0x83);
    let mut calls = Vec::new();
    let rows = collections.project_toy_status(|item_id, is_favorite, has_fanfare| {
        calls.push((item_id, is_favorite, has_fanfare));
        (item_id != 20).then_some((item_id, is_favorite, has_fanfare))
    });
    assert_eq!(calls, vec![(10, false, false), (20, true, true), (30, false, true)]);
    assert_eq!(rows, vec![(10, false, false), (30, false, true)]);
}
