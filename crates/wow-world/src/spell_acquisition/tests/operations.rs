use super::*;
use crate::profession::PrimaryProfessionCapacityPlanLikeCpp;
use crate::trainer_offer::PreparedTrainerOfferLikeCpp;
use std::future::Future;
use std::sync::{Arc, Mutex};

struct Exclusion(Arc<Mutex<Vec<&'static str>>>);
impl Drop for Exclusion {
    fn drop(&mut self) {
        self.0.lock().unwrap().push("released");
    }
}

struct Runtime {
    trace: Arc<Mutex<Vec<&'static str>>>,
    guid: wow_core::ObjectGuid,
    fail_realm_fence: bool,
    pending_commit: bool,
}

impl Runtime {
    fn event(&self, name: &'static str) {
        self.trace.lock().unwrap().push(name);
    }
}

impl PlayerSpellAcquisitionRuntimeLikeCpp for Runtime {
    fn character_guid(&self) -> Option<wow_core::ObjectGuid> {
        Some(self.guid)
    }
    fn has_canonical_player(&self) -> bool {
        true
    }
    fn install_snapshot(
        &mut self,
        _: &PlayerSpellAcquisitionSnapshotLikeCpp,
        _: &BTreeSet<u16>,
    ) -> Result<(), PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp> {
        self.event("installed");
        Ok(())
    }
    fn begin_action_batch(&mut self) {}
    fn record_action(&mut self, _: SpellAcquisitionPostCommitActionLikeCpp) {}
    fn grant_dual_wield(&mut self) -> bool {
        true
    }
    fn publish_action(&mut self, action: &SpellAcquisitionPostCommitActionLikeCpp, _: Option<i32>) {
        if matches!(
            action,
            SpellAcquisitionPostCommitActionLikeCpp::LearnedSpell { .. }
        ) {
            self.event("learned");
        }
    }
}

impl TrainerAcquisitionRuntimeLikeCpp for Runtime {
    type MoneyExclusion = Exclusion;
    async fn commit_acquisition(
        &mut self,
        exclusion: Exclusion,
        prepared: Option<&PreparedPlayerSpellAcquisitionLikeCpp>,
        before: u64,
        after: u64,
    ) -> Option<Exclusion> {
        assert!(prepared.is_some());
        assert_eq!((before, after), (100, 80));
        self.event("attempt");
        if self.pending_commit {
            std::future::pending::<()>().await;
        }
        self.event("committed");
        Some(exclusion)
    }
    fn stage_money(&mut self, _: u64, _: u64) -> bool {
        self.event("money");
        true
    }
    fn publish_money(&mut self, _: u64) {
        self.event("money_packet");
    }
    async fn fence_instance_before_realm(&self) -> bool {
        self.event("instance_fence");
        true
    }
    fn publish_visuals(&self, _: &TrainerAcquisitionPublicationLikeCpp) {
        self.event("visuals");
    }
    async fn fence_realm_before_instance(&self) -> bool {
        self.event("realm_fence");
        !self.fail_realm_fence
    }
    fn publish_skills(&mut self) {
        self.event("skills");
    }
}

fn fixture() -> (
    Runtime,
    PreparedTrainerOfferLikeCpp,
    PlayerSpellAcquisitionSnapshotLikeCpp,
    TrainerAcquisitionPublicationLikeCpp,
) {
    let metadata = MetadataFixture::new(FixtureInput {
        spell_ids: vec![100],
        ..Default::default()
    });
    let guid = wow_core::ObjectGuid::create_player(1, 1);
    let mut source = snapshot();
    source.character_guid = Some(guid);
    let plan = deterministic(project_spell_acquisition_like_cpp(
        &source,
        metadata.metadata(),
        SpellAcquisitionRootLikeCpp::DirectLearn(100),
    ));
    let offer = PreparedTrainerOfferLikeCpp {
        source_spell_id: 100,
        effective_price: 20,
        acquisition_plan: plan,
        profession_plan: PrimaryProfessionCapacityPlanLikeCpp {
            configured_max: 2,
            used_before: 0,
            free_before: 2,
            existing_professions: Vec::new(),
            new_professions: Vec::new(),
            slot_normalizations: Vec::new(),
        },
        battle_pet_species_id: None,
    };
    let publication = TrainerAcquisitionPublicationLikeCpp {
        trainer_guid: guid,
        player_guid: guid,
        trainer_position: wow_core::Position::default(),
        suppress_visuals: false,
    };
    (
        Runtime {
            trace: Arc::new(Mutex::new(Vec::new())),
            guid,
            fail_realm_fence: false,
            pending_commit: false,
        },
        offer,
        source,
        publication,
    )
}

#[tokio::test]
async fn trainer_application_without_session_keeps_guard_through_both_writer_fences() {
    let (mut runtime, offer, source, publication) = fixture();
    let trace = Arc::clone(&runtime.trace);
    let completion = execute_trainer_acquisition_like_cpp(
        &mut runtime,
        Exclusion(Arc::clone(&trace)),
        &offer,
        &source,
        100,
        80,
        &publication,
    )
    .await;
    assert!(matches!(
        completion.result,
        TrainerAcquisitionResultLikeCpp::Applied
    ));
    assert_eq!(
        *trace.lock().unwrap(),
        [
            "attempt",
            "committed",
            "installed",
            "money",
            "money_packet",
            "instance_fence",
            "visuals",
            "realm_fence",
            "learned"
        ]
    );
    drop(completion);
    assert_eq!(trace.lock().unwrap().last(), Some(&"released"));
}

#[tokio::test]
async fn failed_second_fence_retains_exclusion_until_caller_quarantines() {
    let (mut runtime, offer, source, publication) = fixture();
    runtime.fail_realm_fence = true;
    let trace = Arc::clone(&runtime.trace);
    let completion = execute_trainer_acquisition_like_cpp(
        &mut runtime,
        Exclusion(Arc::clone(&trace)),
        &offer,
        &source,
        100,
        80,
        &publication,
    )
    .await;
    assert!(matches!(
        completion.result,
        TrainerAcquisitionResultLikeCpp::WriterFenceFailed
    ));
    assert!(!trace.lock().unwrap().contains(&"released"));
    assert!(!trace.lock().unwrap().contains(&"learned"));
    runtime.event("quarantined");
    drop(completion);
    let trace = trace.lock().unwrap();
    assert_eq!(&trace[trace.len() - 2..], ["quarantined", "released"]);
}

#[tokio::test]
async fn cancelling_pending_purchase_releases_exclusion_without_runtime_publication() {
    let (mut runtime, offer, source, publication) = fixture();
    runtime.pending_commit = true;
    let trace = Arc::clone(&runtime.trace);
    let mut future = Box::pin(execute_trainer_acquisition_like_cpp(
        &mut runtime,
        Exclusion(Arc::clone(&trace)),
        &offer,
        &source,
        100,
        80,
        &publication,
    ));
    assert!(
        std::future::poll_fn(|cx| std::task::Poll::Ready(future.as_mut().poll(cx)))
            .await
            .is_pending()
    );
    assert_eq!(*trace.lock().unwrap(), ["attempt"]);
    drop(future);
    assert_eq!(*trace.lock().unwrap(), ["attempt", "released"]);
}
