// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Session adaptation for the admitted trainer acquisition. Existing money
//! cancellation/recovery and transport writer ownership stay in Session.
use super::*;
use crate::spell_acquisition::{
    PreparedPlayerSpellAcquisitionLikeCpp, TrainerAcquisitionPublicationLikeCpp,
    TrainerAcquisitionRuntimeLikeCpp,
};
use wow_packet::ServerPacket;
use wow_packet::packets::spell::PlaySpellVisualKit;

impl TrainerAcquisitionRuntimeLikeCpp for WorldSession {
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
                self.commit_exclusive_player_money_and_spell_acquisition_like_cpp(
                    exclusion, prepared, before, after,
                )
                .await
            }
            None => {
                self.commit_exclusive_trainer_money_only_like_cpp(exclusion, before, after)
                    .await
            }
        }
    }

    fn stage_money(&mut self, before: u64, after: u64) -> bool {
        self.stage_player_money_change_like_cpp(before, after)
    }

    fn publish_money(&mut self, after: u64) {
        self.send_player_values_update_from_entity_bridge(&[], &[], &[], &[], Some(after));
    }

    async fn fence_instance_before_realm(&self) -> bool {
        self.wait_for_instance_send_before_realm_send_like_cpp()
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
        self.send_packet_realm(&trainer_visual);
        self.broadcast_creature_packet_from_position_to_visible_set_realm_like_cpp(
            publication.trainer_guid,
            publication.trainer_position,
            trainer_visual.to_bytes(),
        );
        self.send_packet_realm(&player_visual);
        self.broadcast_to_movement_set_realm_like_cpp(player_visual.to_bytes(), true);
    }

    async fn fence_realm_before_instance(&self) -> bool {
        self.wait_for_realm_send_before_instance_update_like_cpp()
            .await
    }

    fn publish_skills(&mut self) {
        self.send_complete_player_skill_values_update_like_cpp();
    }
}
