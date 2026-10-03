// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `WorldSession` adapter for the shared spell-acquisition runtime.
//!
//! World retains registry synchronization and state-facing action hooks.
//! Runtime installation and action-packet construction are shared with Trainer
//! through the Application crate.

use std::collections::BTreeSet;

use super::*;

impl PlayerSpellAcquisitionRuntimeLikeCpp for crate::session::WorldSession {
    fn character_guid(&self) -> Option<wow_core::ObjectGuid> {
        self.player_guid()
    }

    fn has_canonical_player(&self) -> bool {
        let (s, h) = crate::session::split_spell_state_ref(self);
        s.has_canonical_player_for_spell_acquisition_like_cpp(h)
    }

    fn install_snapshot(
        &mut self,
        runtime_snapshot: &PlayerSpellAcquisitionSnapshotLikeCpp,
        new_non_durable_skill_tombstone_ids: &BTreeSet<u16>,
    ) -> Result<(), PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp> {
        let owner = self.core.player_acquisition_owner_access_like_cpp();
        let installed = wow_world_application::install_player_spell_acquisition_runtime_snapshot_like_cpp(
            &owner,
            &mut self.spell_state,
            runtime_snapshot,
            new_non_durable_skill_tombstone_ids,
            cfg!(test),
            #[cfg(any(test, feature = "test-fixtures"))]
            (
                &mut self.fixtures.progression.player_skill_test_fixture_like_cpp,
                &mut self.fixtures.progression.represented_enchanting_skill,
            ),
        );
        if installed.is_ok() {
            self.sync_player_registry_state_like_cpp();
        }
        installed
    }

    fn begin_action_batch(&mut self) {
        self.spell_state
            .begin_spell_acquisition_post_commit_action_batch_like_cpp();
    }

    fn record_action(&mut self, action: SpellAcquisitionPostCommitActionLikeCpp) {
        self.spell_state
            .record_spell_acquisition_post_commit_action_like_cpp(action);
    }

    fn grant_dual_wield(&mut self) -> bool {
        let (s, mut h) = crate::session::split_spell_state_mut(self);
        s.grant_dual_wield_after_spell_acquisition_like_cpp(&mut h)
    }

    fn publish_action(
        &mut self,
        action: &SpellAcquisitionPostCommitActionLikeCpp,
        trait_definition_id: Option<i32>,
    ) {
        let owner = self.core.player_acquisition_owner_access_like_cpp();
        wow_world_application::publish_spell_acquisition_action_like_cpp(
            &owner,
            action,
            trait_definition_id,
        );
    }
}
