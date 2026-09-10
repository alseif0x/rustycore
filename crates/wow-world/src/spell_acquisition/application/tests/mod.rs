use super::*;

mod faults;
mod preparation;
mod runtime;
mod validation;

fn make_session() -> (crate::session::WorldSession, flume::Receiver<Vec<u8>>) {
    let (_packet_tx, packet_rx) = flume::bounded(8);
    let (send_tx, send_rx) = flume::unbounded();
    let mut session = crate::session::WorldSession::new(
        1,
        "AcquisitionTest".to_string(),
        0,
        2,
        2,
        54261,
        vec![0; 40],
        "enUS".to_string(),
        packet_rx,
        send_tx,
    );
    session.attach_player_controller_like_cpp(crate::session::SessionPlayerController::new(
        wow_core::ObjectGuid::create_player(1, 42),
        "AcquisitionPlayer".to_string(),
        wow_core::Position::ZERO,
        0,
        1,
        1,
        80,
        0,
    ));
    (session, send_rx)
}

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

fn no_profession_changes() -> PrimaryProfessionCapacityPlanLikeCpp {
    PrimaryProfessionCapacityPlanLikeCpp {
        configured_max: 2,
        used_before: 0,
        free_before: 2,
        existing_professions: Vec::new(),
        new_professions: Vec::new(),
        slot_normalizations: Vec::new(),
    }
}

fn direct_learn_actions(
    spell_id: u32,
    favorite: bool,
    suppress_messaging: bool,
) -> Vec<SpellAcquisitionPostCommitActionLikeCpp> {
    vec![
        SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnOrKnowSpellCriteria { spell_id },
        SpellAcquisitionPostCommitActionLikeCpp::LearnedSpell {
            spell_id,
            favorite,
            suppress_messaging,
        },
        SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnSpellQuestObjective { spell_id },
    ]
}

fn direct_learn_requirements(
    spell_id: u32,
    favorite: bool,
    suppress_messaging: bool,
) -> Vec<SpellAcquisitionPublicationRequirementLikeCpp> {
    vec![
        SpellAcquisitionPublicationRequirementLikeCpp::UpdateLearnOrKnowSpellCriteria { spell_id },
        SpellAcquisitionPublicationRequirementLikeCpp::LearnedSpell {
            spell_id,
            favorite,
            suppress_messaging,
        },
        SpellAcquisitionPublicationRequirementLikeCpp::UpdateLearnSpellQuestObjective { spell_id },
    ]
}

fn direct_learn_plan() -> (
    PlayerSpellAcquisitionSnapshotLikeCpp,
    SpellAcquisitionPlanLikeCpp,
) {
    let source = snapshot(Vec::new());
    let learned = spell(100, PlayerSpellPersistenceStateLikeCpp::New);
    let transition = PlannedSpellTransitionLikeCpp {
        spell_id: 100,
        before: None,
        after: Some(learned),
        provenance: SpellAcquisitionProvenanceLikeCpp::Root {
            root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
        },
    };
    let plan = SpellAcquisitionPlanLikeCpp {
        root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
        source_snapshot: source.clone(),
        mutations: vec![PlannedAcquisitionMutationLikeCpp::Spell(transition.clone())],
        spell_transitions: vec![transition],
        skill_transitions: Vec::new(),
        override_transitions: Vec::new(),
        root_primary_profession_skill_ids: Vec::new(),
        publication_requirements: direct_learn_requirements(100, false, false),
        profession_association_inputs: Vec::new(),
        post_commit_actions: direct_learn_actions(100, false, false),
        diagnostics: Vec::new(),
        resulting_snapshot: snapshot(vec![learned]),
    };
    (source, plan)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum FakeRuntimeEvent {
    Install,
    Begin,
    Record(SpellAcquisitionPostCommitActionLikeCpp),
    GrantDualWield,
    Publish(SpellAcquisitionPostCommitActionLikeCpp, Option<i32>),
}

#[derive(Default)]
struct FakeRuntime {
    character_guid: Option<wow_core::ObjectGuid>,
    canonical_player: bool,
    fail_install: bool,
    dual_wield_grants: usize,
    events: Vec<FakeRuntimeEvent>,
}

impl PlayerSpellAcquisitionRuntimeLikeCpp for FakeRuntime {
    fn character_guid(&self) -> Option<wow_core::ObjectGuid> {
        self.character_guid
    }
    fn has_canonical_player(&self) -> bool {
        self.canonical_player
    }
    fn install_snapshot(
        &mut self,
        _snapshot: &PlayerSpellAcquisitionSnapshotLikeCpp,
        _new_non_durable_skill_tombstone_ids: &BTreeSet<u16>,
    ) -> Result<(), PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp> {
        if self.fail_install {
            return Err(PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp::InvalidPreparedRuntime);
        }
        self.events.push(FakeRuntimeEvent::Install);
        Ok(())
    }
    fn begin_action_batch(&mut self) {
        self.events.push(FakeRuntimeEvent::Begin);
    }
    fn record_action(&mut self, action: SpellAcquisitionPostCommitActionLikeCpp) {
        self.events.push(FakeRuntimeEvent::Record(action));
    }
    fn grant_dual_wield(&mut self) -> bool {
        self.dual_wield_grants += 1;
        self.events.push(FakeRuntimeEvent::GrantDualWield);
        self.canonical_player
    }
    fn publish_action(
        &mut self,
        action: &SpellAcquisitionPostCommitActionLikeCpp,
        trait_definition_id: Option<i32>,
    ) {
        self.events.push(FakeRuntimeEvent::Publish(
            action.clone(),
            trait_definition_id,
        ));
    }
}

fn prepared_direct_learn_for_fake_runtime() -> PreparedPlayerSpellAcquisitionLikeCpp {
    let (source, plan) = direct_learn_plan();
    let PreparedPlayerSpellAcquisitionOutcomeLikeCpp::Ready(prepared) =
        prepare_player_spell_acquisition_like_cpp(&plan, &no_profession_changes(), &source)
            .expect("direct learn plan is valid")
    else {
        panic!("expected ready direct learn plan");
    };
    prepared
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
