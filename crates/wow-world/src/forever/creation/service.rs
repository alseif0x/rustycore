//! Composition-owned character construction; Session retains durability and publication.
use crate::forever::CharacterCatalog;
use wow_packet::forever::character_create::CharacterCreatePayload;
use wow_persistence::forever::creation::save::{
    CreatedCharacter, CreationIdentities, CreationLimits, NewCharacter,
};

/// Application-owned lifecycle adapter for one canonical constructed Player.
/// Neither callback may fail or panic. Cleanup must complete before owner destruction.
pub trait PlayerCreationLifecycle: Send {
    fn committed(&mut self, created: &CreatedCharacter);
    fn cleanup(&mut self);
}

/// Keeps the canonical construction alive independently of its consumed SQL projection.
/// No DTO-only conversion is provided: every successful construction supplies an owner.
pub struct ConstructedCharacter {
    projection: NewCharacter,
    owner: PlayerCreationGuard,
}

impl ConstructedCharacter {
    pub fn new(projection: NewCharacter, owner: Box<dyn PlayerCreationLifecycle>) -> Self {
        Self {
            projection,
            owner: PlayerCreationGuard { owner },
        }
    }

    pub(crate) fn into_parts(self) -> (NewCharacter, PlayerCreationGuard) {
        (self.projection, self.owner)
    }
}

pub(crate) struct PlayerCreationGuard {
    owner: Box<dyn PlayerCreationLifecycle>,
}

impl PlayerCreationGuard {
    pub(crate) fn committed(&mut self, created: &CreatedCharacter) {
        self.owner.committed(created);
    }
}

impl Drop for PlayerCreationGuard {
    fn drop(&mut self) {
        self.owner.cleanup();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CreationFailure {
    pub response_code: u32,
}

impl CreationFailure {
    pub const ERROR: Self = Self { response_code: 25 };
    pub const FAILED: Self = Self { response_code: 26 };
}

pub trait CharacterCreator: Send + Sync {
    fn limits(&self) -> CreationLimits;

    /// On Err, the factory must clean up any partial construction it allocated.
    /// On Ok, the returned guard owns cleanup through save/publication or cancellation.
    fn construct(
        &self,
        identities: &mut dyn CreationIdentities,
        request: &CharacterCreatePayload,
        normalized_name: &str,
        catalog: &CharacterCatalog,
        account_expansion: u8,
        latency: u32,
    ) -> Result<ConstructedCharacter, CreationFailure>;
}
