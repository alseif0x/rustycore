// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Private corpse capability handlers extracted from the legacy misc owner.
//!
//! `#1263 F5 deferred families`: the four registrations moved unchanged to
//! `wow_world_entities::register_corpse_handlers_like_cpp`; `corpse/host.rs`
//! lends that registrar these operations. #1363 slice 4b moved the four
//! bodies to `wow_world_entities::corpse_handlers`; the graveyard condition
//! check stays here because it needs the World player-condition context.

use tracing::warn;
use wow_constants::ConditionType;

#[cfg(test)]
use wow_packet::ClientPacket;
#[cfg(test)]
use wow_packet::packets::misc::PortGraveyard;

mod host;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) async fn handle_request_cemetery_list(&mut self, pkt: wow_packet::WorldPacket) {
        let store = self
            .graveyard_store()
            .cloned()
            .unwrap_or_else(|| std::sync::Arc::new(wow_data::GraveyardStore::default()));
        wow_world_entities::handle_request_cemetery_list_with_catalog_like_cpp(
            self,
            store.as_ref(),
            pkt,
        )
        .await;
    }

    pub(super) fn graveyard_conditions_meet_like_cpp(
        &mut self,
        conditions_ref: &wow_data::ConditionsReference,
    ) -> bool {
        let Some(conditions) = conditions_ref.upgrade() else {
            return true;
        };
        if conditions.is_empty() {
            return true;
        }

        let Some(condition_store) = self.condition_store().cloned() else {
            warn!("Cemetery condition check failed closed: missing condition store");
            return false;
        };
        let Some(player_object) =
            crate::session::hub_ref(self).build_condition_player_object_like_cpp()
        else {
            warn!("Cemetery condition check failed closed: missing player object");
            return false;
        };

        let Some(player_unit_snapshot) =
            crate::session::hub_ref(self).condition_player_unit_snapshot_like_cpp()
        else {
            return false;
        };
        let player_snapshot = crate::session::hub_ref(self).condition_player_snapshot_like_cpp();
        let needs_player_condition_context = conditions.iter().any(|condition| {
            condition.reference_id != 0
                || condition.condition_type == ConditionType::PlayerCondition
        });
        let player_condition_store = needs_player_condition_context
            .then(|| self.player_condition_store().cloned())
            .flatten();
        let player_condition_context = needs_player_condition_context
            .then(|| self.represented_player_condition_context_like_cpp())
            .flatten();

        let mut source_info =
            wow_conditions::ConditionSourceInfo::from_targets(Some(&player_object), None, None);
        source_info.set_unit_target_snapshot(0, player_unit_snapshot);
        source_info.set_player_target_snapshot(0, player_snapshot);
        if let (Some(store), Some(context)) = (
            player_condition_store.as_ref(),
            player_condition_context.as_ref(),
        ) {
            source_info.set_player_condition_store(store.as_ref());
            if let Some(context) = context.as_context(self) {
                source_info.set_player_condition_context(0, context);
            }
        }

        wow_conditions::is_object_meet_to_conditions_like_cpp(
            &mut source_info,
            conditions.as_slice(),
            condition_store.as_ref(),
            |condition, source_info| match wow_conditions::condition_meets_basic_like_cpp(
                condition,
                source_info,
                |current_area, required_area| current_area == required_area,
            ) {
                wow_conditions::ConditionMeetResult::Evaluated(value) => value,
                wow_conditions::ConditionMeetResult::Unsupported => {
                    warn!(
                        "Cemetery condition check failed closed: unsupported {:?}",
                        condition.condition_type
                    );
                    false
                }
            },
        )
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/handlers/entities/corpse/test_only.rs"]
mod test_only;
