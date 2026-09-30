use super::fixtures::*;
// External application scenarios migrated with their original assertions.

use super::fixtures::make_session;
use wow_persistence::{
    PlayerInitialWorldStateRowsLikeCpp, PlayerInitialWorldStateTemplateRowLikeCpp,
    PlayerInitialWorldStateValueRowLikeCpp, PlayerInitialWorldStatesLoadOutcomeLikeCpp,
};
use wow_world::test_fixtures::CollectionLoadPortLikeCpp;

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
        .character_initial_world_states_for_test(571, 0)
        .await;

    assert!(states.contains(&(10, 22)));
    assert!(!states.contains(&(10, 1)));
}

// Synthetic persistence-port failures only; this does not claim C++ DB-failure parity.
#[tokio::test]
async fn initial_world_state_port_preserves_independent_read_failures_like_cpp() {
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
            saved_values: PlayerInitialWorldStateRowsLikeCpp::Failed {
                reason: "character read failed".to_owned(),
            },
        },
        PlayerInitialWorldStatesLoadOutcomeLikeCpp {
            templates: PlayerInitialWorldStateRowsLikeCpp::Failed {
                reason: "world read failed".to_owned(),
            },
            saved_values: PlayerInitialWorldStateRowsLikeCpp::Loaded(vec![
                PlayerInitialWorldStateValueRowLikeCpp { id: 10, value: 22 },
            ]),
        },
    ]);
    let (mut session, _) = make_session();
    session.set_player_lifecycle_port_like_cpp(port);

    let values_failed = session
        .character_initial_world_states_for_test(571, 0)
        .await;
    let templates_failed = session
        .character_initial_world_states_for_test(571, 0)
        .await;

    assert!(values_failed.contains(&(10, 1)));
    assert!(!templates_failed.iter().any(|(id, _)| *id == 10));
}
