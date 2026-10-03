// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use crate::spell_acquisition::{
    PreparedPlayerSpellAcquisitionLikeCpp,
    commit_exclusive_player_money_and_spell_acquisition_like_cpp,
};
use super::{
    TrainerAcquisitionPublicationLikeCpp, TrainerAcquisitionRuntimeLikeCpp,
    context::AppTrainerCx,
};
use wow_packet::ServerPacket;
use wow_packet::packets::spell::PlaySpellVisualKit;
use wow_world_lifecycle::ExclusivePlayerMoneyPersistenceLikeCpp;

impl TrainerAcquisitionRuntimeLikeCpp for AppTrainerCx<'_> {
    type MoneyExclusion = ExclusivePlayerMoneyPersistenceLikeCpp;

    async fn commit_acquisition(
        &mut self,
        exclusion: Self::MoneyExclusion,
        prepared: Option<&PreparedPlayerSpellAcquisitionLikeCpp>,
        before: u64,
        after: u64,
    ) -> Option<Self::MoneyExclusion> {
        match prepared {
            Some(prepared) => {
                let core_access = self.owner.money_transaction();
                #[cfg(any(test, feature = "test-fixtures"))]
                let fixture_result = if self.consumer_test {
                    self.lifecycle.loot_money_persistence_test_result_like_cpp()
                } else {
                    None
                };
                #[cfg(any(test, feature = "test-fixtures"))]
                {
                    return commit_exclusive_player_money_and_spell_acquisition_like_cpp(
                        &*self.lifecycle,
                        core_access,
                        exclusion,
                        prepared,
                        before,
                        after,
                        fixture_result,
                    )
                    .await;
                }
                #[cfg(not(any(test, feature = "test-fixtures")))]
                commit_exclusive_player_money_and_spell_acquisition_like_cpp(
                    &*self.lifecycle,
                    core_access,
                    exclusion,
                    prepared,
                    before,
                    after,
                )
                .await
            }
            None => {
                let mut core_access = self.owner.money_transaction();
                self.lifecycle
                    .commit_exclusive_trainer_money_only_with_access_like_cpp(
                        &mut core_access,
                        exclusion,
                        before,
                        after,
                    )
                    .await
            }
        }
    }

    fn stage_money(&mut self, before: u64, after: u64) -> bool {
        if !self
            .inventory
            .set_player_gold_with_access_like_cpp(&self.owner.inventory(), after)
        {
            return false;
        }
        if before != after {
            self.quest_state
                .enqueue_represented_quest_objective_progress_like_cpp(
                    crate::RepresentedQuestObjectiveProgressEventLikeCpp::MoneyChanged {
                        old_money: before,
                        new_money: after,
                    },
                );
        }
        true
    }

    fn publish_money(&mut self, after: u64) {
        let inventory = self.owner.inventory();
        let publication = self.owner.packet_publication();
        let _ = self
            .inventory
            .send_player_values_update_from_entity_bridge_with_access_like_cpp(
                &inventory,
                &publication,
                self.catalogs.item_store,
                self.catalogs.item_stats_store,
                &[],
                &[],
                &[],
                &[],
                Some(after),
            );
    }

    async fn fence_instance_before_realm(&self) -> bool {
        self.owner
            .packet_publication()
            .wait_for_instance_send_before_realm_send_like_cpp()
            .await
    }

    fn publish_visuals(&self, publication: &TrainerAcquisitionPublicationLikeCpp) {
        if publication.suppress_visuals {
            return;
        }
        let trainer_visual = PlaySpellVisualKit {
            unit: publication.trainer_guid,
            kit_record_id: 179,
            kit_type: 0,
            duration: 0,
            mounted_visual: false,
        };
        let player_visual = PlaySpellVisualKit {
            unit: publication.player_guid,
            kit_record_id: 362,
            kit_type: 1,
            duration: 0,
            mounted_visual: false,
        };
        let packets = self.owner.packet_publication();
        packets.send_packet_realm(&trainer_visual);
        packets.broadcast_from_position_to_visible_set_and_connection_like_cpp(
            publication.trainer_guid,
            publication.trainer_position,
            trainer_visual.to_bytes(),
            true,
            true,
        );
        packets.send_packet_realm(&player_visual);
        packets.broadcast_to_movement_set_in_range_and_connection_like_cpp(
            player_visual.to_bytes(),
            wow_world_core::map_manager::VISIBILITY_RADIUS,
            true,
            #[cfg(any(test, feature = "test-fixtures"))]
            self.fixtures.registry_position,
        );
    }

    async fn fence_realm_before_instance(&self) -> bool {
        self.owner
            .packet_publication()
            .wait_for_realm_send_before_instance_update_like_cpp()
            .await
    }

    fn publish_skills(&mut self) {
        self.owner.publish_complete_skill_values_update_like_cpp(
            self.catalogs.skills,
            self.catalogs.skill_lines,
            self.catalogs.skill_tiers,
            #[cfg(any(test, feature = "test-fixtures"))]
            (
                &self
                    .fixtures
                    .player_skill_fixture
                    .player_skill_records_like_cpp,
                self.fixtures.player_race,
                self.fixtures.player_class,
                self.fixtures.player_level,
            ),
        );
    }
}
