//! One coherent owner capture and an incarnation-bound, single-use acknowledgement.
//! The manager guard never reaches persistence, packet delivery or a callback.
use super::*;

pub(in crate::session) struct PreparedPlayerSave {
    pub request: PlayerCharacterSaveRequestLikeCpp,
    pub header: PlayerSaveToDbSnapshotLikeCpp,
    pub receipt: SavedPlayerReceipt,
}

pub(in crate::session) enum SavedPlayerReceipt {
    Owner(wow_world_core::session::PlayerSaveReceiptLikeCpp),
    #[cfg(test)]
    Fixture {
        expected: PlayerCharacterCommittedGroupsLikeCpp,
        tutorials: Option<PlayerTutorialsSaveLikeCpp>,
    },
}

impl WorldSession {
    pub(in crate::session) fn prepare_player_save_like_cpp(
        &mut self,
        now: i64,
    ) -> Option<PreparedPlayerSave> {
        if self
            .lifecycle
            .durable_loot_money_persistence_tracker_like_cpp()
            .is_indeterminate_like_cpp()
        {
            return None;
        }
        #[cfg(test)]
        if self.core.player_handle_like_cpp.is_none() {
            let header = self.current_player_save_to_db_snapshot_like_cpp()?;
            let request = self.current_player_character_save_request_like_cpp(&header, now)?;
            return Some(PreparedPlayerSave {
                receipt: SavedPlayerReceipt::Fixture {
                    expected: request.committed_groups_like_cpp(),
                    tutorials: request.tutorials.clone(),
                },
                request,
                header,
            });
        }
        let talent_store = self.catalogs.talent_store().map(AsRef::as_ref);
        let spell_store = self
            .catalogs
            .spell_catalogs
            .spell_store()
            .map(AsRef::as_ref);
        let mut owner = self
            .core
            .player_save_operation_access_like_cpp(talent_store, spell_store);
        let inputs = self.lifecycle.player_save_session_inputs_like_cpp(now);
        let captured = owner.capture_like_cpp(inputs)?;
        let (request, header, receipt) = captured.into_parts_like_cpp();
        Some(PreparedPlayerSave {
            request,
            header,
            receipt: SavedPlayerReceipt::Owner(receipt),
        })
    }
}

impl SavedPlayerReceipt {
    /// Consume the receipt only after Applied. Returned adapter flags are intersected
    /// with the actual request, so an unrelated group can never be acknowledged.
    pub(super) fn acknowledge(
        self,
        session: &mut WorldSession,
        committed: &PlayerCharacterCommittedGroupsLikeCpp,
    ) {
        #[cfg(test)]
        match self {
            SavedPlayerReceipt::Fixture {
                expected,
                tutorials,
            } => {
                let groups = committed_groups_like_cpp(&expected, committed);
                session.mark_current_player_save_to_db_committed_like_cpp(&groups);
                let _ = tutorials;
            }
            SavedPlayerReceipt::Owner(receipt) => {
                acknowledge_owner_like_cpp(session, receipt, committed);
            }
        }
        #[cfg(not(test))]
        if let SavedPlayerReceipt::Owner(receipt) = self {
            acknowledge_owner_like_cpp(session, receipt, committed);
        }
    }
}

#[cfg(test)]
fn committed_groups_like_cpp(
    expected: &PlayerCharacterCommittedGroupsLikeCpp,
    committed: &PlayerCharacterCommittedGroupsLikeCpp,
) -> PlayerCharacterCommittedGroupsLikeCpp {
    PlayerCharacterCommittedGroupsLikeCpp {
        player_spells: expected.player_spells && committed.player_spells,
        fallback_player_spells: expected.fallback_player_spells && committed.fallback_player_spells,
        player_skills: expected.player_skills && committed.player_skills,
        equipment_sets: expected.equipment_sets && committed.equipment_sets,
        tutorials_changed: expected.tutorials_changed && committed.tutorials_changed,
        tutorials_insert: expected.tutorials_insert && committed.tutorials_insert,
        reputation: expected.reputation && committed.reputation,
    }
}

fn acknowledge_owner_like_cpp(
    session: &mut WorldSession,
    receipt: wow_world_core::session::PlayerSaveReceiptLikeCpp,
    committed: &PlayerCharacterCommittedGroupsLikeCpp,
) {
    let Some(acknowledged) =
        wow_world_core::session::PlayerSaveOwnerAccessLikeCpp::acknowledge_like_cpp(
            &session.core,
            receipt,
            committed,
        )
    else {
        return;
    };
    if let Some(saved) = acknowledged.tutorials {
        if acknowledged.groups.tutorials_insert {
            session
                .lifecycle
                .set_tutorials_loaded_from_db_like_cpp(true);
        }
        if acknowledged.groups.tutorials_changed
            && session.lifecycle.tutorial_values_like_cpp() == &saved.tutorials
        {
            session.lifecycle.set_tutorials_changed_like_cpp(false);
        }
    }
    if acknowledged.groups.player_spells || acknowledged.groups.player_skills {
        session.sync_player_registry_state_like_cpp();
    }
}
