//! Social handler regressions.
//!
//! Separated from social.rs under #685.

use super::*;
use num_traits::ToPrimitive;
use std::sync::{Arc, Mutex};
use wow_constants::ServerOpcodes;
use wow_persistence::{
    PersistenceFutureLikeCpp, SocialAddCandidateLoadOutcomeLikeCpp, SocialContactLoadRowLikeCpp,
    SocialPartyInviteLookupOutcomeLikeCpp, SocialPersistencePortLikeCpp,
    SocialRelationshipStateLikeCpp,
};

struct RecordingSocialPort {
    contacts: SocialContactListLoadOutcomeLikeCpp,
    mutation: PersistenceOutcomeLikeCpp,
    calls: Mutex<Vec<String>>,
}

impl SocialPersistencePortLikeCpp for RecordingSocialPort {
    fn load_contacts_like_cpp<'a>(
        &'a self,
        player_guid: i64,
        flags: u32,
    ) -> PersistenceFutureLikeCpp<'a, SocialContactListLoadOutcomeLikeCpp> {
        self.calls
            .lock()
            .unwrap()
            .push(format!("load:{player_guid}:{flags}"));
        let outcome = self.contacts.clone();
        Box::pin(async move { outcome })
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
        _target_guid: i64,
        _inviter_guid: i64,
        _inviter_account_id: u32,
    ) -> PersistenceFutureLikeCpp<'a, SocialPartyInviteLookupOutcomeLikeCpp> {
        Box::pin(async { SocialPartyInviteLookupOutcomeLikeCpp::Resolved(false) })
    }

    fn party_invite_target_has_friend_like_cpp<'a>(
        &'a self,
        _target_guid: i64,
        _inviter_guid: i64,
    ) -> PersistenceFutureLikeCpp<'a, SocialPartyInviteLookupOutcomeLikeCpp> {
        Box::pin(async { SocialPartyInviteLookupOutcomeLikeCpp::Resolved(false) })
    }

    fn add_relationship_like_cpp<'a>(
        &'a self,
        _player_guid: i64,
        _target_guid: i64,
        _kind: SocialRelationshipKindLikeCpp,
        _note: String,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        let outcome = self.mutation.clone();
        Box::pin(async move { outcome })
    }

    fn remove_relationship_like_cpp<'a>(
        &'a self,
        player_guid: i64,
        target_guid: i64,
        kind: SocialRelationshipKindLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        self.calls
            .lock()
            .unwrap()
            .push(format!("remove:{player_guid}:{target_guid}:{kind:?}"));
        let outcome = self.mutation.clone();
        Box::pin(async move { outcome })
    }

    fn set_contact_note_like_cpp<'a>(
        &'a self,
        _player_guid: i64,
        _target_guid: i64,
        _note: String,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        let outcome = self.mutation.clone();
        Box::pin(async move { outcome })
    }
}

fn recording_port(
    contacts: SocialContactListLoadOutcomeLikeCpp,
    mutation: PersistenceOutcomeLikeCpp,
) -> Arc<RecordingSocialPort> {
    Arc::new(RecordingSocialPort {
        contacts,
        mutation,
        calls: Mutex::new(Vec::new()),
    })
}

fn make_session() -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (_pkt_tx, pkt_rx) = flume::bounded(8);
    let (send_tx, send_rx) = flume::bounded(8);
    (
        WorldSession::new(
            1,
            "SocialTest".into(),
            0,
            2,
            9,
            54261,
            vec![0; 40],
            "enUS".into(),
            pkt_rx,
            send_tx,
        ),
        send_rx,
    )
}

fn opcode(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}

mod scenarios;
