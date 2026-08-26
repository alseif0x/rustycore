// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Auxiliary Player-login reads cross the typed lifecycle port.

use super::*;

use std::collections::VecDeque;
use std::sync::Mutex;
use wow_packet::packets::update::ChrCustomizationChoiceValuesUpdate;
use wow_persistence::{
    AccountCollectionLoadOutcomeLikeCpp, AccountCollectionLoadRequestLikeCpp,
    AccountCollectionSaveLikeCpp, PersistenceFutureLikeCpp, PersistenceOutcomeLikeCpp,
    PlayerCharacterSaveRequestLikeCpp, PlayerCharacterSaveResultLikeCpp,
    PlayerCustomizationLoadRowLikeCpp, PlayerHomebindPersistenceRequestLikeCpp,
    PlayerInstanceTimeRestrictionLoadRowLikeCpp, PlayerLifecyclePortLikeCpp,
    PlayerLoginAuxiliaryLoadOutcomeLikeCpp, PlayerLoginAuxiliaryLoadRequestLikeCpp,
    PlayerLoginAuxiliaryLoadedLikeCpp, PlayerOfflineMarkLikeCpp, PlayerSpellChargeLoadRowLikeCpp,
    PlayerSpellCooldownLoadRowLikeCpp,
};

struct AuxiliaryLoadPortLikeCpp {
    requests: Mutex<Vec<PlayerLoginAuxiliaryLoadRequestLikeCpp>>,
    outcomes: Mutex<VecDeque<PlayerLoginAuxiliaryLoadOutcomeLikeCpp>>,
}

impl AuxiliaryLoadPortLikeCpp {
    fn new(
        outcomes: impl IntoIterator<Item = PlayerLoginAuxiliaryLoadOutcomeLikeCpp>,
    ) -> Arc<Self> {
        Arc::new(Self {
            requests: Mutex::new(Vec::new()),
            outcomes: Mutex::new(outcomes.into_iter().collect()),
        })
    }

    fn requests(&self) -> Vec<PlayerLoginAuxiliaryLoadRequestLikeCpp> {
        self.requests.lock().unwrap().clone()
    }
}

impl PlayerLifecyclePortLikeCpp for AuxiliaryLoadPortLikeCpp {
    fn mark_offline_like_cpp<'a>(
        &'a self,
        _mark: PlayerOfflineMarkLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async {
            PersistenceOutcomeLikeCpp::Failed {
                reason: "auxiliary-load-only fixture".to_owned(),
            }
        })
    }

    fn persist_homebind_like_cpp<'a>(
        &'a self,
        _request: PlayerHomebindPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async {
            PersistenceOutcomeLikeCpp::Failed {
                reason: "auxiliary-load-only fixture".to_owned(),
            }
        })
    }

    fn load_account_collection_like_cpp<'a>(
        &'a self,
        _request: AccountCollectionLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, AccountCollectionLoadOutcomeLikeCpp> {
        Box::pin(async {
            AccountCollectionLoadOutcomeLikeCpp::Failed {
                reason: "auxiliary-load-only fixture".to_owned(),
            }
        })
    }

    fn load_login_auxiliary_like_cpp<'a>(
        &'a self,
        request: PlayerLoginAuxiliaryLoadRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerLoginAuxiliaryLoadOutcomeLikeCpp> {
        self.requests.lock().unwrap().push(request);
        let outcome = self
            .outcomes
            .lock()
            .unwrap()
            .pop_front()
            .expect("one typed auxiliary outcome per request");
        Box::pin(async move { outcome })
    }

    fn save_account_collection_like_cpp<'a>(
        &'a self,
        _save: AccountCollectionSaveLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async {
            PersistenceOutcomeLikeCpp::Failed {
                reason: "auxiliary-load-only fixture".to_owned(),
            }
        })
    }

    fn save_character_like_cpp<'a>(
        &'a self,
        request: PlayerCharacterSaveRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PlayerCharacterSaveResultLikeCpp> {
        let committed = request.committed_groups_like_cpp();
        Box::pin(async move {
            PlayerCharacterSaveResultLikeCpp {
                outcome: PersistenceOutcomeLikeCpp::Failed {
                    reason: "auxiliary-load-only fixture".to_owned(),
                },
                committed,
            }
        })
    }
}

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

#[tokio::test]
async fn spell_history_loads_preserve_cpp_order_expiry_and_charge_aggregation() {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let port = AuxiliaryLoadPortLikeCpp::new([
        PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
            PlayerLoginAuxiliaryLoadedLikeCpp::SpellCooldowns(vec![
                PlayerSpellCooldownLoadRowLikeCpp {
                    spell_id: 133,
                    item_id: 6948,
                    cooldown_end: now + 60,
                    category_id: 12,
                    category_end: now + 30,
                },
                PlayerSpellCooldownLoadRowLikeCpp {
                    spell_id: 134,
                    item_id: 0,
                    cooldown_end: now - 1,
                    category_id: 0,
                    category_end: 0,
                },
            ]),
        ),
        PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
            PlayerLoginAuxiliaryLoadedLikeCpp::SpellCharges(vec![
                PlayerSpellChargeLoadRowLikeCpp {
                    category_id: 42,
                    recharge_start: now - 5,
                    recharge_end: now + 45,
                },
                PlayerSpellChargeLoadRowLikeCpp {
                    category_id: 42,
                    recharge_start: now - 10,
                    recharge_end: now + 30,
                },
                PlayerSpellChargeLoadRowLikeCpp {
                    category_id: 7,
                    recharge_start: now - 20,
                    recharge_end: now - 1,
                },
            ]),
        ),
    ]);
    let (mut session, _, _) = make_session();
    session.set_player_lifecycle_port_like_cpp(port.clone());
    let guid = ObjectGuid::create_player(1, 46);

    let (history, charges) = session
        .load_character_spell_history_packets_like_cpp(guid)
        .await;

    assert_eq!(history.len(), 1);
    assert_eq!(history[0].spell_id, 133);
    assert_eq!(history[0].item_id, 6948);
    assert_eq!(history[0].category, 12);
    assert_eq!(charges.len(), 1);
    assert_eq!(charges[0].category, 42);
    assert_eq!(charges[0].consumed_charges, 2);
    assert!(charges[0].next_recovery_time_ms > 0);
    assert!(session.represented_character_spell_cooldowns_loaded_like_cpp);
    assert!(session.represented_character_spell_charges_loaded_like_cpp);
    assert_eq!(
        session.represented_character_spell_cooldowns_like_cpp.len(),
        1
    );
    assert_eq!(
        session.represented_character_spell_charges_like_cpp[&42].len(),
        2
    );
    assert_eq!(
        port.requests(),
        vec![
            PlayerLoginAuxiliaryLoadRequestLikeCpp::SpellCooldowns { player_guid: 46 },
            PlayerLoginAuxiliaryLoadRequestLikeCpp::SpellCharges { player_guid: 46 },
        ]
    );
}

#[tokio::test]
async fn spell_history_cooldown_failure_does_not_suppress_independent_charges_like_cpp() {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let port = AuxiliaryLoadPortLikeCpp::new([
        PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed {
            reason: "cooldown query failed".to_owned(),
        },
        PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
            PlayerLoginAuxiliaryLoadedLikeCpp::SpellCharges(vec![
                PlayerSpellChargeLoadRowLikeCpp {
                    category_id: 42,
                    recharge_start: now,
                    recharge_end: now + 60,
                },
            ]),
        ),
    ]);
    let (mut session, _, _) = make_session();
    session.set_player_lifecycle_port_like_cpp(port);

    let (history, charges) = session
        .load_character_spell_history_packets_like_cpp(ObjectGuid::create_player(1, 47))
        .await;

    assert!(history.is_empty());
    assert_eq!(charges.len(), 1);
    assert!(!session.represented_character_spell_cooldowns_loaded_like_cpp);
    assert!(session.represented_character_spell_charges_loaded_like_cpp);
}
