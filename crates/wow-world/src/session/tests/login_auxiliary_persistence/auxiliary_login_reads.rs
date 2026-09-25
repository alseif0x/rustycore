use super::*;

#[tokio::test]
async fn auxiliary_login_reads_preserve_cpp_row_and_publication_rules() {
    let port = AuxiliaryLoadPortLikeCpp::new([
        PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
            PlayerLoginAuxiliaryLoadedLikeCpp::Customizations(vec![
                PlayerCustomizationLoadRowLikeCpp {
                    option_id: 12,
                    choice_id: 34,
                },
                PlayerCustomizationLoadRowLikeCpp {
                    option_id: 0,
                    choice_id: 0,
                },
            ]),
        ),
        PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
            PlayerLoginAuxiliaryLoadedLikeCpp::CompletedAchievements(vec![9001, 9001, 0, 9002]),
        ),
        PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
            PlayerLoginAuxiliaryLoadedLikeCpp::InstanceTimeRestrictions(vec![
                PlayerInstanceTimeRestrictionLoadRowLikeCpp {
                    instance_id: 10,
                    release_time: 100,
                },
                PlayerInstanceTimeRestrictionLoadRowLikeCpp {
                    instance_id: 10,
                    release_time: 200,
                },
                PlayerInstanceTimeRestrictionLoadRowLikeCpp {
                    instance_id: 0,
                    release_time: 0,
                },
            ]),
        ),
    ]);
    let (mut session, _, _) = make_session();
    session.set_player_lifecycle_port_like_cpp(port.clone());
    let guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(guid));

    let customizations = session.load_player_customizations_like_cpp(guid).await;
    session.load_completed_achievements_like_cpp().await;
    session.load_instance_time_restrictions_like_cpp().await;

    assert_eq!(
        customizations,
        vec![
            ChrCustomizationChoiceValuesUpdate {
                option_id: 12,
                choice_id: 34,
            },
            ChrCustomizationChoiceValuesUpdate {
                option_id: 0,
                choice_id: 0,
            },
        ]
    );
    assert_eq!(
        session.represented_completed_achievements_like_cpp,
        HashSet::from([9001, 9002])
    );
    assert_eq!(
        session.represented_instance_reset_times_like_cpp,
        BTreeMap::from([(0, 0), (10, 100)])
    );
    assert_eq!(
        port.requests(),
        vec![
            PlayerLoginAuxiliaryLoadRequestLikeCpp::Customizations { player_guid: 42 },
            PlayerLoginAuxiliaryLoadRequestLikeCpp::CompletedAchievements { player_guid: 42 },
            PlayerLoginAuxiliaryLoadRequestLikeCpp::InstanceTimeRestrictions { account_id: 1 },
        ]
    );
}

#[tokio::test]
async fn empty_auxiliary_login_rows_clear_stale_represented_state() {
    let port = AuxiliaryLoadPortLikeCpp::new([
        PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
            PlayerLoginAuxiliaryLoadedLikeCpp::Customizations(Vec::new()),
        ),
        PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
            PlayerLoginAuxiliaryLoadedLikeCpp::CompletedAchievements(Vec::new()),
        ),
        PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
            PlayerLoginAuxiliaryLoadedLikeCpp::InstanceTimeRestrictions(Vec::new()),
        ),
    ]);
    let (mut session, _, _) = make_session();
    session.set_player_lifecycle_port_like_cpp(port);
    let guid = ObjectGuid::create_player(1, 43);
    session.set_player_guid(Some(guid));
    session
        .represented_completed_achievements_like_cpp
        .insert(7);
    session
        .represented_instance_reset_times_like_cpp
        .insert(8, 9);

    assert!(
        session
            .load_player_customizations_like_cpp(guid)
            .await
            .is_empty()
    );
    session.load_completed_achievements_like_cpp().await;
    session.load_instance_time_restrictions_like_cpp().await;

    assert!(
        session
            .represented_completed_achievements_like_cpp
            .is_empty()
    );
    assert!(session.represented_instance_reset_times_like_cpp.is_empty());
}

#[tokio::test]
async fn failed_auxiliary_login_reads_do_not_publish_or_preserve_stale_values() {
    let failed = || PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed {
        reason: "fixture query failure".to_owned(),
    };
    let port = AuxiliaryLoadPortLikeCpp::new([failed(), failed(), failed()]);
    let (mut session, _, _) = make_session();
    session.set_player_lifecycle_port_like_cpp(port);
    let guid = ObjectGuid::create_player(1, 44);
    session.set_player_guid(Some(guid));
    session
        .represented_completed_achievements_like_cpp
        .insert(7);
    session
        .represented_instance_reset_times_like_cpp
        .insert(8, 9);

    assert!(
        session
            .load_player_customizations_like_cpp(guid)
            .await
            .is_empty()
    );
    session.load_completed_achievements_like_cpp().await;
    session.load_instance_time_restrictions_like_cpp().await;

    assert!(
        session
            .represented_completed_achievements_like_cpp
            .is_empty()
    );
    assert!(session.represented_instance_reset_times_like_cpp.is_empty());
}

#[tokio::test]
async fn missing_auxiliary_login_port_means_unknown_and_clears_session_caches() {
    let (mut session, _, _) = make_session();
    let guid = ObjectGuid::create_player(1, 45);
    session.set_player_guid(Some(guid));
    session
        .represented_completed_achievements_like_cpp
        .insert(7);
    session
        .represented_instance_reset_times_like_cpp
        .insert(8, 9);

    assert!(
        session
            .load_player_customizations_like_cpp(guid)
            .await
            .is_empty()
    );
    session.load_completed_achievements_like_cpp().await;
    session.load_instance_time_restrictions_like_cpp().await;

    assert!(
        session
            .represented_completed_achievements_like_cpp
            .is_empty()
    );
    assert!(session.represented_instance_reset_times_like_cpp.is_empty());
}
