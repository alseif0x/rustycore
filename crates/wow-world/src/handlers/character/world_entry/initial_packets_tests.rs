use crate::handlers::test_support::world::make_session;
use crate::test_fixtures::CollectionLoadPortLikeCpp;
use wow_persistence::{
    PlayerInitialWorldStateRowsLikeCpp, PlayerInitialWorldStateTemplateRowLikeCpp,
    PlayerInitialWorldStateValueRowLikeCpp, PlayerInitialWorldStatesLoadOutcomeLikeCpp,
};

#[tokio::test]
async fn initial_world_state_port_applies_saved_overlay_after_templates_like_cpp() {
    let port = CollectionLoadPortLikeCpp::for_initial_world_states([
        PlayerInitialWorldStatesLoadOutcomeLikeCpp {
            templates: PlayerInitialWorldStateRowsLikeCpp::Loaded(vec![
                PlayerInitialWorldStateTemplateRowLikeCpp {
                    id: 10,
                    default_value: 1,
                    map_ids_csv: String::new(),
                    area_ids_csv: String::new(),
                },
            ]),
            saved_values: PlayerInitialWorldStateRowsLikeCpp::Loaded(vec![
                PlayerInitialWorldStateValueRowLikeCpp { id: 10, value: 22 },
            ]),
        },
    ]);
    let (mut session, _) = make_session();
    session.set_player_lifecycle_port_like_cpp(port);

    let states = session
        .test_load_initial_world_states_for_login_like_cpp(571, 0)
        .await;

    assert!(states.contains(&(10, 22)));
    assert!(!states.contains(&(10, 1)));
}
