// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Recovery helpers for persisted character locations and partial login.

use super::*;

impl WorldSession {
    /// Retry the final C++ `Player::LoadFromDB` recovery location after the
    /// saved map cannot be selected. C++ first tries go-back/map-entrance
    /// triggers; Rust's current MapEntry/instance-template stores do not expose
    /// enough data to select those faithfully, so this implements the final
    /// mandatory homebind retry and reports whether relocation succeeded.
    pub(in crate::handlers::character) fn resolved_homebind_area_id_like_cpp(
        &self,
        map_id: u32,
        position: Position,
    ) -> u32 {
        let map_area_id = self
            .map_store()
            .as_deref()
            .map(|store| u32::from(store.area_table_id_like_cpp(map_id)))
            .unwrap_or(0);
        zone_and_area_for_position_like_cpp(
            &self.mmap_runtime_config_like_cpp().data_dir,
            map_id,
            position.x,
            position.y,
            self.area_table_store().map(|store| store.as_ref()),
            |map_id| {
                self.map_store()
                    .as_deref()
                    .map(|store| u32::from(store.area_table_id_like_cpp(map_id)))
                    .unwrap_or(0)
            },
        )
        .ok()
        .map(|(_, area_id)| area_id)
        .filter(|area_id| *area_id != 0)
        .unwrap_or(map_area_id)
    }

    pub(in crate::handlers::character) async fn delete_invalid_character_homebind_like_cpp(
        &self,
        guid: ObjectGuid,
    ) {
        let Some(port) = self.player_lifecycle_port_like_cpp() else {
            return;
        };
        match port
            .persist_homebind_like_cpp(
                wow_persistence::PlayerHomebindPersistenceRequestLikeCpp::DeleteInvalid {
                    player_guid: guid.counter() as u64,
                },
            )
            .await
        {
            wow_persistence::PersistenceOutcomeLikeCpp::Applied { .. } => {}
            wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason }
            | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } => warn!(
                player_guid = guid.counter(),
                "failed to delete invalid character homebind like C++ Player::_LoadHomeBind: {reason}"
            ),
        }
    }

    pub(in crate::handlers::character) fn seed_login_location_zone_area_like_cpp(
        &mut self,
        zone_id: &mut i32,
        location: CharacterLoginLocationLikeCpp,
    ) {
        let resolved = login_location_zone_area_like_cpp(location, |map_id, position| {
            zone_and_area_for_position_like_cpp(
                &self.mmap_runtime_config_like_cpp().data_dir,
                map_id,
                position.x,
                position.y,
                self.area_table_store().map(|store| store.as_ref()),
                |map_id| {
                    self.map_store()
                        .as_deref()
                        .map(|store| u32::from(store.area_table_id_like_cpp(map_id)))
                        .unwrap_or(0)
                },
            )
        });
        let fallback_area_id = location.bind_area_id.unwrap_or_else(|| {
            self.map_store()
                .as_deref()
                .map(|store| u32::from(store.area_table_id_like_cpp(location.map_id)))
                .unwrap_or(0)
        });
        let (fallback_zone_id, fallback_area_id) = zone_and_area_from_area_id_like_cpp(
            fallback_area_id,
            self.area_table_store().map(Arc::as_ref),
        );

        match resolved {
            Ok((resolved_zone_id, resolved_area_id)) if resolved_area_id != 0 => {
                *zone_id = i32::try_from(resolved_zone_id)
                    .expect("resolved login zone ID must fit the packet field");
                self.set_player_zone_area_like_cpp(resolved_zone_id, resolved_area_id);
            }
            Ok(_) => {
                *zone_id = i32::try_from(fallback_zone_id)
                    .expect("fallback login zone ID must fit the packet field");
                self.set_player_zone_area_like_cpp(fallback_zone_id, fallback_area_id);
                warn!(
                    map_id = location.map_id,
                    x = location.position.x,
                    y = location.position.y,
                    fallback_zone_id,
                    fallback_area_id,
                    "terrain returned no fallback login area for C++ UpdatePositionData"
                );
            }
            Err(error) => {
                *zone_id = i32::try_from(fallback_zone_id)
                    .expect("fallback login zone ID must fit the packet field");
                self.set_player_zone_area_like_cpp(fallback_zone_id, fallback_area_id);
                warn!(
                    map_id = location.map_id,
                    x = location.position.x,
                    y = location.position.y,
                    %error,
                    fallback_zone_id,
                    fallback_area_id,
                    "failed to refresh fallback login zone/area like C++ UpdatePositionData"
                );
            }
        }
    }

    pub(in crate::handlers::character) fn retry_login_at_homebind_like_cpp(
        &mut self,
        map_id: &mut i32,
        zone_id: &mut i32,
        position: &mut Position,
        homebind: CharacterLoginLocationLikeCpp,
    ) -> bool {
        if self.current_canonical_player_map_key_like_cpp().is_some() {
            return false;
        }
        if !usable_character_homebind_like_cpp(
            homebind,
            self.map_store().map(Arc::as_ref),
            self.expansion,
        ) {
            return false;
        }
        let homebind_map_id =
            u16::try_from(homebind.map_id).expect("validated character login homebind map ID");

        *map_id = i32::from(homebind_map_id);
        *position = homebind.position;
        self.seed_login_location_zone_area_like_cpp(zone_id, homebind);
        self.set_player_map_position_like_cpp(homebind_map_id, homebind.position);
        let _ = self.ensure_canonical_world_map_for_current_player_like_cpp();
        self.current_canonical_player_map_key_like_cpp().is_some()
    }

    /// A failed cross-socket ordering fence means the successful-login burst
    /// cannot be completed coherently. C++ loses the socket and destroys the
    /// partially loaded `Player`; mirror that lifetime boundary immediately
    /// so the process-wide character claim cannot outlive this failed login.
    pub(in crate::handlers::character) fn abort_partial_login_sequence_like_cpp(&mut self) {
        let retirement = self.cleanup_shared_runtime_state();
        if matches!(
            retirement,
            crate::FinalizationOutcome::Applied | crate::FinalizationOutcome::NoWork
        ) {
            self.set_player_guid(None);
        }
        self.kick("WorldSession::HandlePlayerLogin login packet sequence failed");
    }

    /// C++ `Player::LoadFromDB` first attempts go-back/homebind relocation and
    /// never substitutes an arbitrary instance when authoritative map
    /// selection fails. The caller has already tried the represented BG entry
    /// point/homebind recovery; if that also produced no canonical map, fail
    /// closed: tear down the partially loaded player while its GUID is still
    /// present, then disconnect without `CharacterLoginFailed`, matching C++'s
    /// final `LoadFromDB == false` cleanup path.
    pub(in crate::handlers::character) fn continue_login_after_grid_load_like_cpp(
        &mut self,
        guid: ObjectGuid,
        map_id: i32,
        instance_id: u32,
        outcome: Option<crate::session::PlayerGridLoadOutcomeLikeCpp>,
    ) -> bool {
        if outcome.is_some_and(|outcome| !outcome.map_unavailable) {
            return true;
        }

        let reason = if outcome.is_some() {
            "authoritative canonical map unavailable"
        } else {
            "loaded-grid resolver unavailable"
        };
        warn!(
            guid = ?guid,
            map_id,
            instance_id,
            reason,
            "RUST_LOGIN map_add aborted"
        );
        let retirement = self.cleanup_shared_runtime_state();
        if matches!(
            retirement,
            crate::FinalizationOutcome::Applied | crate::FinalizationOutcome::NoWork
        ) {
            self.set_player_guid(None);
        }
        self.kick("WorldSession::HandlePlayerLogin authoritative map resolution failed");
        false
    }
}

impl WorldSession {
    async fn persist_repaired_character_homebind_like_cpp(
        &self,
        guid: ObjectGuid,
        homebind: CharacterLoginLocationLikeCpp,
    ) {
        let Some(bind_area_id) = homebind.bind_area_id else {
            return;
        };
        let Ok(map_id) = u16::try_from(homebind.map_id) else {
            return;
        };
        let Ok(bind_area_id) = u16::try_from(bind_area_id) else {
            return;
        };

        let Some(port) = self.player_lifecycle_port_like_cpp() else {
            return;
        };
        match port
            .persist_homebind_like_cpp(
                wow_persistence::PlayerHomebindPersistenceRequestLikeCpp::InsertRepaired {
                    player_guid: guid.counter() as u64,
                    map_id,
                    area_id: bind_area_id,
                    x: homebind.position.x,
                    y: homebind.position.y,
                    z: homebind.position.z,
                    orientation: homebind.position.orientation,
                },
            )
            .await
        {
            wow_persistence::PersistenceOutcomeLikeCpp::Applied { .. } => {}
            wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason }
            | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } => warn!(
                player_guid = guid.counter(),
                "failed to persist repaired character homebind like C++ Player::_LoadHomeBind: {reason}"
            ),
        }
    }

    pub(in crate::handlers::character) async fn repair_character_homebind_like_cpp(
        &self,
        guid: ObjectGuid,
        race: u8,
        player_create_info: PlayerCreateInfoLikeCpp,
        create_mode: u8,
        first_login: bool,
    ) -> Option<CharacterLoginLocationLikeCpp> {
        let mut replacement = if first_login {
            first_login_creation_homebind_like_cpp(player_create_info, create_mode)
        } else {
            None
        };
        if replacement.is_none() {
            replacement = self.load_default_graveyard_homebind_like_cpp(race);
        }
        let mut replacement = replacement?;
        replacement.bind_area_id =
            Some(self.resolved_homebind_area_id_like_cpp(replacement.map_id, replacement.position));

        self.persist_repaired_character_homebind_like_cpp(guid, replacement)
            .await;
        Some(replacement)
    }
}
