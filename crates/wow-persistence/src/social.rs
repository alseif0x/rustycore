//! Contact lookup and relationship mutation persistence capabilities.
//! Mechanical relocation from lib.rs in #578; public crate-root paths are retained.

use crate::{PersistenceFutureLikeCpp, PersistenceOutcomeLikeCpp};

/// Which bit in one `character_social` row the gameplay owner is addressing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocialRelationshipKindLikeCpp {
    Friend,
    Ignored,
}

/// SQLx-free projection of one contact-list row. Online visibility remains a
/// runtime concern, so this carries only durable character/social metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SocialContactLoadRowLikeCpp {
    pub friend_guid: i64,
    pub type_flags: u32,
    pub note: String,
    pub class_id: u32,
    pub level: u32,
    pub zone_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SocialContactListLoadOutcomeLikeCpp {
    Loaded(Vec<SocialContactLoadRowLikeCpp>),
    Failed { reason: String },
}

/// Candidate metadata needed before Session can apply the C++ self/faction
/// gates. Durable list state is deliberately loaded only after those gates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SocialAddCandidateLikeCpp {
    pub guid: i64,
    pub race: u8,
    pub class_id: u32,
    pub level: i32,
    pub zone_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SocialAddCandidateLoadOutcomeLikeCpp {
    Found(SocialAddCandidateLikeCpp),
    NotFound,
    Failed { reason: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SocialRelationshipStateLikeCpp {
    pub already_present: bool,
    pub relationship_count: i64,
}

/// Classified result of one C++ party-invite social membership check.
/// A read has no ambiguous-COMMIT state, but driver failure must remain
/// distinct so the gameplay caller can preserve its current fail-open path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SocialPartyInviteLookupOutcomeLikeCpp {
    Resolved(bool),
    Failed { reason: String },
}

/// SQLx-free Characters-database capability for the represented social-list
/// reads and writes. Packet construction and gameplay admission stay outside.
pub trait SocialPersistencePortLikeCpp: Send + Sync {
    fn load_contacts_like_cpp<'a>(
        &'a self,
        player_guid: i64,
        flags: u32,
    ) -> PersistenceFutureLikeCpp<'a, SocialContactListLoadOutcomeLikeCpp>;

    fn load_add_candidate_like_cpp<'a>(
        &'a self,
        normalized_name: String,
        kind: SocialRelationshipKindLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, SocialAddCandidateLoadOutcomeLikeCpp>;

    fn load_relationship_state_like_cpp<'a>(
        &'a self,
        player_guid: i64,
        target_guid: i64,
        kind: SocialRelationshipKindLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, SocialRelationshipStateLikeCpp>;

    /// Represent C++ `invitedPlayer->GetSocial()->HasIgnore(...)` while the
    /// canonical in-memory social map is not yet available cross-session.
    fn party_invite_target_ignores_like_cpp<'a>(
        &'a self,
        target_guid: i64,
        inviter_guid: i64,
        inviter_account_id: u32,
    ) -> PersistenceFutureLikeCpp<'a, SocialPartyInviteLookupOutcomeLikeCpp>;

    /// Represent C++ `invitedPlayer->GetSocial()->HasFriend(...)`. The caller
    /// invokes this only after the ignore gate and low-level check.
    fn party_invite_target_has_friend_like_cpp<'a>(
        &'a self,
        target_guid: i64,
        inviter_guid: i64,
    ) -> PersistenceFutureLikeCpp<'a, SocialPartyInviteLookupOutcomeLikeCpp>;

    fn add_relationship_like_cpp<'a>(
        &'a self,
        player_guid: i64,
        target_guid: i64,
        kind: SocialRelationshipKindLikeCpp,
        note: String,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp>;

    fn remove_relationship_like_cpp<'a>(
        &'a self,
        player_guid: i64,
        target_guid: i64,
        kind: SocialRelationshipKindLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp>;

    fn set_contact_note_like_cpp<'a>(
        &'a self,
        player_guid: i64,
        target_guid: i64,
        note: String,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp>;
}
