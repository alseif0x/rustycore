// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character create/delete/rename/customise, corpse and resurrection.

use super::*;

impl WorldSession {
    /// Update the realmcharacters count in the login database.
    ///
    /// Counts how many characters this account has on the character DB, then
    /// upserts the count into `realmcharacters` in the login DB.
    pub(crate) async fn update_realm_characters(&self) {
        let port = match self
            .lifecycle
            .player_lifecycle_port_like_cpp()
            .map(Arc::clone)
        {
            Some(port) => port,
            None => return,
        };
        let request = wow_persistence::PlayerRealmCharacterCountRefreshRequestLikeCpp {
            account_id: self.core.account_id,
            realm_id: self.realm_id() as u32,
        };
        match port.refresh_realm_character_count_like_cpp(request).await {
            wow_persistence::PersistenceOutcomeLikeCpp::Applied { .. } => {
                debug!(
                    "Updated realmcharacters: account={} realm={}",
                    self.core.account_id,
                    self.realm_id()
                );
            }
            wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason }
            | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } => {
                warn!("Failed to update realmcharacters: {reason}");
            }
        }
    }

    /// Apply only on the owning Session, never from a database worker.
    pub(crate) fn enqueue_character_rename_like_cpp(
        &self,
        guid: ObjectGuid,
        outcome: &crate::character_administration::RenameOutcome,
    ) -> flume::r#async::SendFut<'static, Vec<u8>> {
        use crate::character_administration::RenameFailure;
        let result = if outcome.result.is_ok() {
            RESPONSE_SUCCESS_LIKE_CPP
        } else {
            CHAR_CREATE_ERROR_LIKE_CPP
        };
        let delivery = self
            .send_tx()
            .clone()
            .into_send_async(wow_packet::ServerPacket::to_bytes(&CharacterRenameResult {
                result,
                name: outcome.new_name.clone(),
                guid: (result == RESPONSE_SUCCESS_LIKE_CPP).then_some(guid),
            }));
        // Confirmed DB outcome is logged once, not on each poll of a pending send.
        match &outcome.result {
            Ok(old_name) => {
                info!(
                    "Account {} renamed character {:?} from {} to {}",
                    self.core.account_id, guid, old_name, outcome.new_name
                );
            }
            Err(failure) => match failure {
                RenameFailure::QueryFailed(reason) => {
                    warn!("Character rename free-name query failed: {reason}");
                }
                RenameFailure::CommitFailed(reason) => {
                    warn!("Character rename transaction failed: {reason}");
                }
                RenameFailure::NotFound | RenameFailure::NotEligible => {}
            },
        };
        delivery
    }

    /// CMSG_HEARTH_AND_RESURRECT — battlefield hearth/resurrection escape.
    /// C++ ref: `WorldSession::HandleHearthAndResurrect`.
    pub async fn handle_hearth_and_resurrect(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = HearthAndResurrect::read(&mut pkt) {
            warn!(
                account = self.core.account_id,
                "HearthAndResurrect parse failed: {error}"
            );
            return;
        }

        if crate::session::hub_ref(self).resolved_is_in_taxi_flight_like_cpp() != Some(false) {
            return;
        }

        let Some((_, area_id)) = crate::session::hub_ref(self).player_zone_area_like_cpp() else {
            return;
        };
        let Some(area_table_store) = self.catalogs.area_table_store() else {
            debug!(
                account = self.core.account_id,
                area_id, "HearthAndResurrect ignored without represented AreaTableStore"
            );
            return;
        };
        let Some(area_entry) = area_table_store.get(area_id) else {
            return;
        };
        if !area_entry.allow_hearth_and_resurrect_from_area_like_cpp() {
            return;
        }

        // C++ first lets Battlefield own the leave flow when one exists. Rust
        // has no battlefield manager attached to WorldSession yet, so this
        // represented branch covers the AreaTable/homebind path only.
        crate::session::hub_mut(self).apply_represented_resurrection_percent_like_cpp(1.0);
        if let Some(homebind) = self.represented_homebind_like_cpp() {
            self.teleport_to(homebind.map_id, homebind.position).await;
        }
    }

    /// C++ `Player::LoadFromDB`: `CHAR_SEL_CHARACTER_CUSTOMIZATIONS`.
    ///
    /// The rows are copied into `PlayerData::Customizations` before
    /// `PlayerData::WriteCreate`, and each element serializes as
    /// `(ChrCustomizationOptionID, ChrCustomizationChoiceID)` uint32s.
    pub(crate) async fn load_player_customizations_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Vec<ChrCustomizationChoiceValuesUpdate> {
        let Some(port) = self
            .lifecycle
            .player_lifecycle_port_like_cpp()
            .map(Arc::clone)
        else {
            return Vec::new();
        };

        let rows = match port
            .load_login_auxiliary_like_cpp(
                wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::Customizations {
                    player_guid: guid.counter() as u64,
                },
            )
            .await
        {
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::Customizations(rows),
            ) => rows,
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    player_guid = guid.counter(),
                    "Failed to load character customizations: {reason}"
                );
                return Vec::new();
            }
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(_) => {
                warn!(
                    player_guid = guid.counter(),
                    "Player lifecycle port returned the wrong auxiliary login data for customizations"
                );
                return Vec::new();
            }
        };
        let customizations = rows
            .into_iter()
            .map(|row| ChrCustomizationChoiceValuesUpdate {
                option_id: row.option_id,
                choice_id: row.choice_id,
            })
            .collect::<Vec<_>>();

        info!(
            player_guid = guid.counter(),
            customizations = customizations.len(),
            "Loaded character customizations like C++"
        );
        customizations
    }

    pub(super) fn load_default_graveyard_homebind_like_cpp(
        &self,
        race: u8,
    ) -> Option<CharacterLoginLocationLikeCpp> {
        let [primary_safe_loc_id, neutral_pandaren_safe_loc_id] =
            default_graveyard_safe_loc_ids_for_race_like_cpp(race);
        let primary_safe_loc_id = primary_safe_loc_id?;
        let store = self.catalogs.world_safe_loc_store_like_cpp()?;
        store
            .get(primary_safe_loc_id)
            .or_else(|| neutral_pandaren_safe_loc_id.and_then(|id| store.get(id)))
            .map(|safe_loc| CharacterLoginLocationLikeCpp {
                map_id: safe_loc.map_id,
                bind_area_id: None,
                position: safe_loc.position,
            })
    }

    pub(super) async fn load_map_corpse_data_like_cpp(
        &self,
        map_id: u16,
        instance_id: u32,
    ) -> MapCorpseLoadOutcomeLikeCpp {
        let (state, hub) = crate::session::split_lifecycle_ref(self);
        state
            .load_map_corpse_data_like_cpp(hub, map_id, instance_id)
            .await
    }
}
