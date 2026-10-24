//! One coherent owner capture and an incarnation-bound, single-use acknowledgement.
//! The manager guard never reaches persistence, packet delivery or a callback.
use super::*;

pub(in crate::session) struct PreparedPlayerSave {
    pub request: PlayerCharacterSaveRequestLikeCpp,
    pub header: PlayerSaveToDbSnapshotLikeCpp,
    pub receipt: SavedPlayerReceipt,
}

pub(in crate::session) enum SavedPlayerReceipt {
    Owner,
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
        let chr_races_store = self.catalogs.chr.races_store.as_deref();
        let difficulty_store = self.catalogs.difficulty_store.as_deref();
        let mut owner = self.core.player_save_operation_access_like_cpp(
            talent_store,
            spell_store,
            chr_races_store,
            difficulty_store,
        );
        let inputs = self.lifecycle.player_save_session_inputs_like_cpp(now);
        let captured = owner.capture_like_cpp(inputs)?;
        let (request, header, _receipt) = captured.into_parts_like_cpp();
        Some(PreparedPlayerSave {
            request,
            header,
            receipt: SavedPlayerReceipt::Owner,
        })
    }

    #[cfg(test)]
    pub(crate) fn player_save_snapshot_blocked_for_test(&mut self, now: i64) -> bool {
        self.prepare_player_save_like_cpp(now).is_none()
    }
}

#[cfg(test)]
pub(in crate::session) fn committed_groups_like_cpp(
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
