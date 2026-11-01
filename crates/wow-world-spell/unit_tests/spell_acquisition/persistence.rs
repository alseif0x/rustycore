// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::collections::BTreeMap;

use super::*;
use wow_spell_acquisition::{
    PlayerAcquisitionLifecycleLikeCpp, PlayerSkillAcquisitionRowLikeCpp,
    PlayerSpellAcquisitionRowLikeCpp, ProfessionAssociationInputLikeCpp,
};

fn snapshot(
    spells: Vec<PlayerSpellAcquisitionRowLikeCpp>,
) -> PlayerSpellAcquisitionSnapshotLikeCpp {
    PlayerSpellAcquisitionSnapshotLikeCpp {
        character_guid: Some(wow_core::ObjectGuid::create_player(1, 42)),
        spells,
        skills: Vec::new(),
        occupied_skill_slots: 0,
        overrides: Vec::new(),
        primary_profession_skill_ids: Vec::new(),
        non_durable_skill_tombstone_ids: Vec::new(),
        race: 1,
        class: 1,
        level: 80,
        lifecycle: PlayerAcquisitionLifecycleLikeCpp::InWorld,
        future_player_condition_resolutions: Vec::new(),
        cast_resolutions: BTreeMap::new(),
    }
}

fn spell(
    spell_id: u32,
    state: PlayerSpellPersistenceStateLikeCpp,
) -> PlayerSpellAcquisitionRowLikeCpp {
    PlayerSpellAcquisitionRowLikeCpp {
        spell_id,
        active: true,
        disabled: false,
        dependent: false,
        favorite: false,
        trait_definition_id: None,
        state,
    }
}

#[test]
fn stable_source_authority_is_exact_and_rejects_unsaved_state() {
    let mut favorite = spell(200, PlayerSpellPersistenceStateLikeCpp::Unchanged);
    favorite.favorite = true;
    let mut dependent = spell(201, PlayerSpellPersistenceStateLikeCpp::Unchanged);
    dependent.dependent = true;
    dependent.favorite = true;
    let mut source = snapshot(vec![favorite, dependent]);
    source.skills = vec![PlayerSkillAcquisitionRowLikeCpp {
        skill_id: 164,
        step: 1,
        value: 75,
        maximum: 150,
        profession_association: ProfessionAssociationInputLikeCpp::Slot(1),
        state: PlayerSkillPersistenceStateLikeCpp::Unchanged,
    }];
    source.occupied_skill_slots = 1;

    assert_eq!(
        stable_source_durable_authority_like_cpp(&source),
        Some(DurablePlayerSpellAcquisitionAuthorityLikeCpp {
            spells: vec![DurablePlayerSpellRowLikeCpp {
                spell_id: 200,
                active: true,
                disabled: false,
            }],
            favorite_spell_ids: vec![200, 201],
            skills: vec![DurablePlayerSkillRowLikeCpp {
                skill_id: 164,
                value: 75,
                maximum: 150,
                profession_slot: 1,
            }],
        })
    );

    let mut unsaved_spell = source.clone();
    unsaved_spell.spells[0].state = PlayerSpellPersistenceStateLikeCpp::Changed;
    assert!(stable_source_durable_authority_like_cpp(&unsaved_spell).is_none());
    assert!(snapshot_has_pending_durable_save_like_cpp(&unsaved_spell));

    let mut with_temporary_spell = source.clone();
    with_temporary_spell
        .spells
        .push(spell(202, PlayerSpellPersistenceStateLikeCpp::Temporary));
    assert_eq!(
        stable_source_durable_authority_like_cpp(&with_temporary_spell),
        stable_source_durable_authority_like_cpp(&source)
    );
    assert!(!snapshot_has_pending_durable_save_like_cpp(
        &with_temporary_spell
    ));

    let mut unsaved_skill = source;
    unsaved_skill.skills[0].state = PlayerSkillPersistenceStateLikeCpp::New;
    assert!(stable_source_durable_authority_like_cpp(&unsaved_skill).is_none());
    assert!(snapshot_has_pending_durable_save_like_cpp(&unsaved_skill));
}

struct RecordingSpellAcquisitionPort {
    attempt: wow_persistence::PlayerSpellAcquisitionPersistenceAttemptLikeCpp,
    reconciliation: PlayerSpellAcquisitionMoneyReconciliationLikeCpp,
    reconciliation_calls: std::sync::atomic::AtomicUsize,
}

impl PlayerSpellAcquisitionPersistencePortLikeCpp for RecordingSpellAcquisitionPort {
    fn attempt_player_spell_acquisition_like_cpp(
        &self,
        _request: PlayerSpellAcquisitionPersistenceRequestLikeCpp,
    ) -> wow_persistence::PersistenceFutureLikeCpp<
        '_,
        wow_persistence::PlayerSpellAcquisitionPersistenceAttemptLikeCpp,
    > {
        Box::pin(std::future::ready(self.attempt.clone()))
    }
    fn reconcile_player_spell_acquisition_like_cpp(
        &self,
        _request: PlayerSpellAcquisitionPersistenceRequestLikeCpp,
    ) -> wow_persistence::PersistenceFutureLikeCpp<
        '_,
        PlayerSpellAcquisitionMoneyReconciliationLikeCpp,
    > {
        self.reconciliation_calls
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Box::pin(std::future::ready(self.reconciliation))
    }
}

fn empty_persistence_request() -> PlayerSpellAcquisitionPersistenceRequestLikeCpp {
    let authority = DurablePlayerSpellAcquisitionAuthorityLikeCpp {
        spells: Vec::new(),
        favorite_spell_ids: Vec::new(),
        skills: Vec::new(),
    };
    PlayerSpellAcquisitionPersistenceRequestLikeCpp {
        player_guid: 42,
        money_before: 100,
        money_after: 80,
        operation_token: [7; 16],
        source_authority: authority.clone(),
        resulting_authority: authority,
        operations: Vec::new(),
    }
}

#[tokio::test]
async fn port_outcome_reconciles_only_an_unknown_commit_like_cpp() {
    use std::sync::atomic::Ordering;
    use wow_persistence::PlayerSpellAcquisitionPersistenceAttemptLikeCpp as Attempt;

    let applied = RecordingSpellAcquisitionPort {
        attempt: Attempt::Applied,
        reconciliation: PlayerSpellAcquisitionMoneyReconciliationLikeCpp::Indeterminate,
        reconciliation_calls: std::sync::atomic::AtomicUsize::new(0),
    };
    assert!(matches!(
        persist_player_spell_acquisition_through_port_like_cpp(
            &applied,
            empty_persistence_request()
        )
        .await,
        PlayerSpellAcquisitionPersistenceOutcomeLikeCpp::Applied
    ));
    assert_eq!(applied.reconciliation_calls.load(Ordering::Relaxed), 0);

    let unknown = RecordingSpellAcquisitionPort {
        attempt: Attempt::CommitOutcomeUnknown {
            reason: "lost reply".to_owned(),
        },
        reconciliation: PlayerSpellAcquisitionMoneyReconciliationLikeCpp::Committed,
        reconciliation_calls: std::sync::atomic::AtomicUsize::new(0),
    };
    assert!(matches!(
        persist_player_spell_acquisition_through_port_like_cpp(
            &unknown,
            empty_persistence_request()
        )
        .await,
        PlayerSpellAcquisitionPersistenceOutcomeLikeCpp::ReconciledCommit(reason)
            if reason == "lost reply"
    ));
    assert_eq!(unknown.reconciliation_calls.load(Ordering::Relaxed), 1);
}
