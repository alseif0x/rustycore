//! Represented battle-pet state at the Session boundary.
//!
//! Moved out of the Session root under #607. The durable add and experience
//! operations and the journal helpers moved to
//! `wow_world_inventory::battle_pet_session` (#1363 slice 7b); the represented
//! test bodies live in `handlers/battle_pet_test_shims.rs`.

use super::*;

impl WorldSession {
    /// Attach this session to the one canonical journal owner for its
    /// Battle.net account. The attachment releases any held journal lease on
    /// drop, matching C++ `WorldSession` teardown.
    pub fn set_battle_pet_account_attachment_like_cpp(
        &mut self,
        attachment: BattlePetAccountAttachmentLikeCpp,
    ) {
        self.core
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        self.lifecycle
            .set_battle_pet_account_attachment_like_cpp(attachment);
    }
    /// C++ `BattlePetMgr::SendJournalLockStatus`, represented as the successful
    /// local acquisition path until the global world journal-lock owner exists.
    ///
    /// Thin World wrapper over the application owner's shared helper (#1263 F5);
    /// the login path (`handlers/character/world_entry.rs`) still calls it.
    pub(crate) async fn send_battle_pet_journal_lock_status_like_cpp(&mut self) {
        let (lifecycle, mut hub) = crate::session::split_battle_pet_handler_mut(self);
        wow_world_inventory::send_battle_pet_journal_lock_status_like_cpp(
            &mut hub,
            lifecycle,
            cfg!(test),
        )
        .await;
    }
    /// C++ `BattlePetMgr::HasJournalLock`.
    ///
    /// Thin World wrapper over the application owner's shared helper (#1263 F5);
    /// the battle-pet spell-effect admission path
    /// (the `wow_world_spell` cast checks, #1363 slice 9e) calls it as a host method.
    pub(crate) fn has_represented_battle_pet_journal_lock_like_cpp(&self) -> bool {
        wow_world_inventory::has_represented_battle_pet_journal_lock_like_cpp(
            crate::session::hub_ref(self),
            &self.lifecycle,
            cfg!(test),
        )
    }
    /// One represented battle pet from the canonical owner or the World-test
    /// fixture.
    ///
    /// Thin World wrapper over the application owner's shared helper (#1263 F5);
    /// the battle-pet spell-effect admission path
    /// (the `wow_world_spell` cast checks, #1363 slice 9e) calls it as a host method.
    pub(crate) fn represented_battle_pet_like_cpp(
        &self,
        pet_guid: ObjectGuid,
    ) -> Option<RepresentedBattlePetDataLikeCpp> {
        wow_world_inventory::represented_battle_pet_like_cpp(
            crate::session::hub_ref(self),
            &self.lifecycle,
            cfg!(test),
            pet_guid,
        )
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/pets/battle_pet/f3_shims.rs"]
mod f3_shims;
