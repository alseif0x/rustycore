use super::RenameRequest;
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;
use wow_persistence::{
    CharacterAdministrationLoadOutcomeLikeCpp as LoadOutcome,
    CharacterAdministrationMutationOutcomeLikeCpp as MutationOutcome,
    CharacterAdministrationPersistencePortLikeCpp,
    CharacterCreatePersistenceRequestLikeCpp, CharacterCustomizationPersistenceLikeCpp,
    CharacterCustomizeCandidateLikeCpp, CharacterRenameCandidateLikeCpp, PersistenceFutureLikeCpp,
};

pub type RenameCandidateFixtureLikeCpp = LoadOutcome<CharacterRenameCandidateLikeCpp>;

pub struct RenamePersistencePortFixtureLikeCpp {
    candidate: Mutex<Option<oneshot::Receiver<RenameCandidateFixtureLikeCpp>>>,
    commits: Mutex<Vec<(u64, String, u16)>>,
}

impl RenamePersistencePortFixtureLikeCpp {
    pub fn commit_snapshot(&self) -> Vec<(u64, String, u16)> {
        self.commits.lock().unwrap().clone()
    }

    pub fn commit_count(&self) -> usize {
        self.commits.lock().unwrap().len()
    }
}

impl CharacterAdministrationPersistencePortLikeCpp for RenamePersistencePortFixtureLikeCpp {
    fn find_character_name_like_cpp(
        &self,
        _: &str,
    ) -> PersistenceFutureLikeCpp<'_, LoadOutcome<()>> {
        panic!("not a rename operation")
    }

    fn load_account_character_count_like_cpp(
        &self,
        _: u32,
    ) -> PersistenceFutureLikeCpp<'_, LoadOutcome<u64>> {
        panic!("not a rename operation")
    }

    fn create_character_like_cpp(
        &self,
        _: CharacterCreatePersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, MutationOutcome> {
        panic!("not a rename operation")
    }

    fn delete_owned_character_like_cpp(
        &self,
        _: u64,
        _: u32,
    ) -> PersistenceFutureLikeCpp<'_, MutationOutcome> {
        panic!("not a rename operation")
    }

    fn load_rename_candidate_like_cpp(
        &self,
        guid: u64,
        name: &str,
    ) -> PersistenceFutureLikeCpp<'_, RenameCandidateFixtureLikeCpp> {
        assert_eq!((guid, name), (42, "Newname"));
        let completion = self.candidate.lock().unwrap().take().expect("one read");
        Box::pin(async move { completion.await.expect("controlled read completion") })
    }

    fn commit_rename_like_cpp(
        &self,
        guid: u64,
        name: &str,
        flags: u16,
    ) -> PersistenceFutureLikeCpp<'_, MutationOutcome> {
        self.commits
            .lock()
            .unwrap()
            .push((guid, name.into(), flags));
        Box::pin(async { MutationOutcome::Applied })
    }

    fn load_customize_candidate_like_cpp(
        &self,
        _: u64,
    ) -> PersistenceFutureLikeCpp<'_, LoadOutcome<CharacterCustomizeCandidateLikeCpp>> {
        panic!("not a rename operation")
    }

    fn commit_customize_like_cpp(
        &self,
        _: u64,
        _: &str,
        _: u16,
        _: Vec<CharacterCustomizationPersistenceLikeCpp>,
    ) -> PersistenceFutureLikeCpp<'_, MutationOutcome> {
        panic!("not a rename operation")
    }
}

pub fn fixture() -> (
    Arc<RenamePersistencePortFixtureLikeCpp>,
    oneshot::Sender<RenameCandidateFixtureLikeCpp>,
    RenameRequest,
) {
    let (send, receive) = oneshot::channel();
    (
        Arc::new(RenamePersistencePortFixtureLikeCpp {
            candidate: Mutex::new(Some(receive)),
            commits: Mutex::new(Vec::new()),
        }),
        send,
        RenameRequest {
            guid: 42,
            new_name: "Newname".into(),
        },
    )
}

pub fn candidate() -> RenameCandidateFixtureLikeCpp {
    LoadOutcome::Loaded(CharacterRenameCandidateLikeCpp {
        old_name: "Oldname".into(),
        at_login_flags: 0x009,
    })
}
