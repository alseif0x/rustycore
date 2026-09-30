use super::*;

#[test]
fn has_item_appearance_reports_permanent_before_temporary_like_cpp() {
    let mut fixture = AdmissionFixture::default();

    assert_eq!(fixture.player.gameplay_state().collections.has_appearance(65), (false, false));

    fixture.player.gameplay_state_mut().collections.temporary_item_appearances
        .insert(65, HashSet::from([ObjectGuid::create_item(1, 900)]));
    assert_eq!(fixture.player.gameplay_state().collections.has_appearance(65), (true, true));

    fixture.player.gameplay_state_mut().collections.item_appearances.insert(65);
    assert_eq!(fixture.player.gameplay_state().collections.has_appearance(65), (true, false));
}

#[test]
fn load_account_item_appearances_rebuilds_blocks_and_favorites_like_cpp() {
    let mut fixture = AdmissionFixture::default();
    fixture.player.gameplay_state_mut().collections.item_appearances.insert(999);
    fixture.player.gameplay_state_mut().collections.favorite_item_appearances
        .insert(998, FavoriteAppearanceStateLikeCpp::New);

    fixture.load_appearances(
        [(2, 1_u32), (0, (1_u32 << 1) | (1_u32 << 31)), (1, 0_u32)],
        [65, 96],
    );

    assert_eq!(
        fixture.player.gameplay_state_mut().collections.item_appearances,
        HashSet::from([1, 31, 64])
    );
    assert_eq!(
        fixture.player.gameplay_state().collections.active_appearance_blocks(|| None),
        vec![(1_u32 << 1) | (1_u32 << 31), 0, 1]
    );
    assert_eq!(
        fixture.player.gameplay_state().collections.favorite_item_appearances_like_cpp().get(&65).copied(),
        Some(FavoriteAppearanceStateLikeCpp::Unchanged)
    );
    assert_eq!(
        fixture.player.gameplay_state().collections.favorite_item_appearances_like_cpp().get(&96).copied(),
        Some(FavoriteAppearanceStateLikeCpp::Unchanged)
    );
    assert_eq!(
        fixture.player.gameplay_state().collections.favorite_item_appearances_like_cpp().get(&998).copied(),
        None
    );
}

#[test]
fn account_item_appearance_save_plan_matches_collection_mgr_state_transitions_like_cpp() {
    let mut fixture = AdmissionFixture::default();
    fixture.player.gameplay_state_mut().collections.item_appearances = HashSet::from([1, 31, 64]);
    fixture.player.gameplay_state_mut().collections.favorite_item_appearances
        .insert(65, FavoriteAppearanceStateLikeCpp::New);
    fixture.player.gameplay_state_mut().collections.favorite_item_appearances
        .insert(96, FavoriteAppearanceStateLikeCpp::Removed);
    fixture.player.gameplay_state_mut().collections.favorite_item_appearances
        .insert(97, FavoriteAppearanceStateLikeCpp::Unchanged);

    let plan = fixture.player.gameplay_state_mut().collections.appearance_save_plan();

    assert_eq!(
        plan.appearance_blocks,
        vec![(0, (1_u32 << 1) | (1_u32 << 31)), (2, 1_u32)]
    );
    assert_eq!(plan.favorite_inserts, vec![65]);
    assert_eq!(plan.favorite_deletes, vec![96]);
    assert_eq!(
        fixture.player.gameplay_state().collections.favorite_item_appearances_like_cpp().get(&65).copied(),
        Some(FavoriteAppearanceStateLikeCpp::Unchanged)
    );
    assert_eq!(
        fixture.player.gameplay_state().collections.favorite_item_appearances_like_cpp().get(&96).copied(),
        None
    );
    assert_eq!(
        fixture.player.gameplay_state().collections.favorite_item_appearances_like_cpp().get(&97).copied(),
        Some(FavoriteAppearanceStateLikeCpp::Unchanged)
    );
}

#[test]
fn load_account_transmog_illusions_includes_static_defaults_like_cpp() {
    let mut fixture = AdmissionFixture::default();
    fixture.player.gameplay_state_mut().collections.transmog_illusions.insert(999);

    fixture.load_illusions([(0, 1_u32 << 1), (2, 1_u32 << 3)]);

    assert!(fixture.player.gameplay_state().collections.has_transmog_illusion(1));
    assert!(fixture.player.gameplay_state().collections.has_transmog_illusion(67));
    for illusion_id in DEFAULT_TRANSMOG_ILLUSIONS_LIKE_CPP {
        assert!(fixture.player.gameplay_state().collections.has_transmog_illusion(illusion_id));
    }
    assert!(!fixture.player.gameplay_state().collections.has_transmog_illusion(999));
}

#[test]
fn account_transmog_illusion_save_plan_writes_non_empty_blocks_like_cpp() {
    let mut fixture = AdmissionFixture::default();
    fixture.load_illusions([(2, 1_u32 << 3)]);

    let plan = fixture.player.gameplay_state().collections.illusion_save_plan();

    assert_eq!(
        plan.illusion_blocks,
        vec![
            (
                0,
                (1_u32 << 3) | (1_u32 << 13) | (1_u32 << 22) | (1_u32 << 23),
            ),
            (1, (1_u32 << 2) | (1_u32 << 11) | (1_u32 << 12)),
            (2, 1_u32 << 3),
        ]
    );
    assert!(!plan.is_empty());
}
