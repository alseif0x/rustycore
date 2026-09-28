use std::sync::{Arc, Mutex};
use wow_persistence::{
    PersistenceFutureLikeCpp, PersistenceOutcomeLikeCpp, SocialAddCandidateLoadOutcomeLikeCpp,
    SocialContactListLoadOutcomeLikeCpp, SocialPartyInviteLookupOutcomeLikeCpp,
    SocialPersistencePortLikeCpp, SocialRelationshipKindLikeCpp, SocialRelationshipStateLikeCpp,
};

pub struct PartyInviteSocialPortLikeCpp {
    ignore: SocialPartyInviteLookupOutcomeLikeCpp,
    friend: SocialPartyInviteLookupOutcomeLikeCpp,
    calls: Mutex<Vec<String>>,
}

impl PartyInviteSocialPortLikeCpp {
    pub fn new(
        ignore: SocialPartyInviteLookupOutcomeLikeCpp,
        friend: SocialPartyInviteLookupOutcomeLikeCpp,
    ) -> Arc<Self> {
        Arc::new(Self {
            ignore,
            friend,
            calls: Mutex::new(Vec::new()),
        })
    }

    pub fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
}

impl SocialPersistencePortLikeCpp for PartyInviteSocialPortLikeCpp {
    fn load_contacts_like_cpp<'a>(
        &'a self,
        _player_guid: i64,
        _flags: u32,
    ) -> PersistenceFutureLikeCpp<'a, SocialContactListLoadOutcomeLikeCpp> {
        Box::pin(async { SocialContactListLoadOutcomeLikeCpp::Loaded(Vec::new()) })
    }

    fn load_add_candidate_like_cpp<'a>(
        &'a self,
        _normalized_name: String,
        _kind: SocialRelationshipKindLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, SocialAddCandidateLoadOutcomeLikeCpp> {
        Box::pin(async { SocialAddCandidateLoadOutcomeLikeCpp::NotFound })
    }

    fn load_relationship_state_like_cpp<'a>(
        &'a self,
        _player_guid: i64,
        _target_guid: i64,
        _kind: SocialRelationshipKindLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, SocialRelationshipStateLikeCpp> {
        Box::pin(async {
            SocialRelationshipStateLikeCpp {
                already_present: false,
                relationship_count: 0,
            }
        })
    }

    fn party_invite_target_ignores_like_cpp<'a>(
        &'a self,
        target_guid: i64,
        inviter_guid: i64,
        inviter_account_id: u32,
    ) -> PersistenceFutureLikeCpp<'a, SocialPartyInviteLookupOutcomeLikeCpp> {
        self.calls.lock().unwrap().push(format!(
            "ignore:{target_guid}:{inviter_guid}:{inviter_account_id}"
        ));
        let outcome = self.ignore.clone();
        Box::pin(async move { outcome })
    }

    fn party_invite_target_has_friend_like_cpp<'a>(
        &'a self,
        target_guid: i64,
        inviter_guid: i64,
    ) -> PersistenceFutureLikeCpp<'a, SocialPartyInviteLookupOutcomeLikeCpp> {
        self.calls
            .lock()
            .unwrap()
            .push(format!("friend:{target_guid}:{inviter_guid}"));
        let outcome = self.friend.clone();
        Box::pin(async move { outcome })
    }

    fn add_relationship_like_cpp<'a>(
        &'a self,
        _player_guid: i64,
        _target_guid: i64,
        _kind: SocialRelationshipKindLikeCpp,
        _note: String,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async { PersistenceOutcomeLikeCpp::Applied { rows: 0 } })
    }

    fn remove_relationship_like_cpp<'a>(
        &'a self,
        _player_guid: i64,
        _target_guid: i64,
        _kind: SocialRelationshipKindLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async { PersistenceOutcomeLikeCpp::Applied { rows: 0 } })
    }

    fn set_contact_note_like_cpp<'a>(
        &'a self,
        _player_guid: i64,
        _target_guid: i64,
        _note: String,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async { PersistenceOutcomeLikeCpp::Applied { rows: 0 } })
    }
}
