use crate::handlers::test_support::world::make_session;
use crate::test_fixtures::CollectionLoadPortLikeCpp;
use wow_persistence::{
    AccountCollectionLoadOutcomeLikeCpp, AccountCollectionLoadedLikeCpp,
    AccountCollectionRowsLikeCpp,
};

#[tokio::test]
async fn account_item_appearance_load_preserves_independent_query_failure_like_cpp() {
    let port = CollectionLoadPortLikeCpp::new([AccountCollectionLoadOutcomeLikeCpp::Loaded(
        AccountCollectionLoadedLikeCpp::ItemAppearances {
            appearance_blocks: AccountCollectionRowsLikeCpp::Failed {
                reason: "appearance read failed".to_owned(),
            },
            favorite_appearance_ids: AccountCollectionRowsLikeCpp::Loaded(vec![91]),
        },
    )]);
    let (mut session, _) = make_session();
    session.set_battlenet_account_id(77);
    session.set_player_lifecycle_port_like_cpp(port);

    session.load_account_item_appearances_like_cpp().await;

    assert!(
        session
            .account_transmog_active_player_rows_like_cpp()
            .is_empty()
    );
    assert!(!session.set_appearance_is_favorite_like_cpp(91, true));
}
