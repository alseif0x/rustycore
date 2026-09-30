//! Unit health/death state and the existing revision authority operations.

use super::*;

impl Unit {
    pub(crate) fn set_type(&mut self, type_id: TypeId, type_mask: TypeMask) {
        self.world.object_mut().set_type(type_id, type_mask);
    }
    pub const fn data(&self) -> &UnitDataValues {
        &self.data
    }
    pub const fn death_state(&self) -> DeathState {
        self.death_state
    }
    pub fn set_death_state(&mut self, state: DeathState) {
        if self.death_state == state {
            return;
        }
        self.death_state = state;
        self.advance_health_state_revision_like_cpp();
    }
    pub const fn health_state_revision_like_cpp(&self) -> u64 {
        self.health_state_revision_like_cpp.value
    }
    pub fn health_state_revision_authority_like_cpp(&self) -> HealthStateRevisionAuthorityLikeCpp {
        self.health_state_revision_like_cpp.authority.clone()
    }
    pub fn shares_health_state_revision_authority_like_cpp(
        &self,
        authority: &HealthStateRevisionAuthorityLikeCpp,
    ) -> bool {
        self.health_state_revision_like_cpp
            .authority
            .shares_storage_like_cpp(authority)
    }
    /// Copy the canonical health tuple into a temporary whole-entity snapshot
    /// immediately before that snapshot replaces the canonical object.
    ///
    /// This deliberately copies the revision exactly: the authoritative state
    /// did not change, only the container object did. Callers must never use
    /// this on a live owner to replay an older mirror transition.
    pub fn preserve_authoritative_health_state_for_snapshot_like_cpp(
        &mut self,
        authoritative: &Self,
    ) {
        self.set_u64_field(
            UNIT_DATA_MAX_HEALTH_BIT,
            authoritative.data.max_health,
            |data| &mut data.max_health,
        );
        self.set_u64_field(UNIT_DATA_HEALTH_BIT, authoritative.data.health, |data| {
            &mut data.health
        });
        self.death_state = authoritative.death_state;
        self.health_state_revision_like_cpp = authoritative.health_state_revision_like_cpp.clone();
    }
    /// Mark a mirror replay as the exact already-committed health transition.
    ///
    /// Replaying normal setters is intentional because it runs the mirror's
    /// local death hooks, but those setters reserve fresh sequence values. The
    /// caller may invoke this only after verifying that the resulting
    /// health/death tuple and incarnation metadata exactly match the canonical
    /// commit. The shared allocator never moves backward; only this mirror's
    /// local state version is aligned with the committed version.
    pub fn adopt_committed_health_state_revision_for_mirror_like_cpp(
        &mut self,
        committed_revision: u64,
    ) {
        assert_ne!(
            committed_revision, 0,
            "a committed health transition must carry a nonzero revision"
        );
        self.health_state_revision_like_cpp
            .authority
            .allocator
            .fetch_max(committed_revision, Ordering::AcqRel);
        self.health_state_revision_like_cpp.value = committed_revision;
    }
    pub(in crate::unit) fn advance_health_state_revision_like_cpp(&mut self) {
        self.health_state_revision_like_cpp.advance();
    }
    pub const fn is_alive(&self) -> bool {
        matches!(self.death_state, DeathState::Alive)
    }
    pub const fn is_dead(&self) -> bool {
        matches!(self.death_state, DeathState::Dead | DeathState::Corpse)
    }
}
