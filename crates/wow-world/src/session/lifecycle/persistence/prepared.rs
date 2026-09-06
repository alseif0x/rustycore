//! One coherent owner capture and an incarnation-bound, single-use acknowledgement.
//! The manager guard never reaches persistence, packet delivery or a callback.
use super::*;
use crate::session::loaded_character_power_snapshot_like_cpp;

pub(in crate::session) struct PreparedPlayerSave {
    pub request: PlayerCharacterSaveRequestLikeCpp,
    pub header: PlayerSaveToDbSnapshotLikeCpp,
    pub receipt: SavedPlayerReceipt,
}

pub(in crate::session) struct SavedPlayerReceipt {
    handle: Option<wow_map::PlayerHandle>,
    owner: Option<wow_entities::PlayerSaveAcknowledgementLikeCpp>,
    expected: PlayerCharacterCommittedGroupsLikeCpp,
    tutorials: Option<PlayerTutorialsSaveLikeCpp>,
}

impl WorldSession {
    pub(in crate::session) fn prepare_player_save_like_cpp(
        &mut self,
        now: i64,
    ) -> Option<PreparedPlayerSave> {
        if self
            .durable_loot_money_persistence_like_cpp
            .is_indeterminate_like_cpp()
        {
            return None;
        }
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            let header = self.current_player_save_to_db_snapshot_like_cpp()?;
            let request = self.current_player_character_save_request_like_cpp(&header, now)?;
            return Some(PreparedPlayerSave {
                receipt: SavedPlayerReceipt {
                    expected: request.committed_groups_like_cpp(),
                    tutorials: request.tutorials.clone(),
                    handle: None,
                    owner: None,
                },
                request,
                header,
            });
        }
        let handle = self.player_handle_like_cpp?;
        if self.player_guid()? != handle.guid() {
            return None;
        }
        let manager = self.canonical_map_manager.as_ref()?.lock().ok()?;
        let residence = manager.player_residence_like_cpp(handle)?;
        manager.with_player_like_cpp(handle, |player| {
            let header = self.player_save_header_from_owner_like_cpp(player, residence);
            if player.teleport_state_like_cpp().recovery
                == wow_entities::PlayerTransferRecovery::Terminal
            {
                let world = player.unit().world();
                if !world.position().is_valid_map_coord_like_cpp()
                    || u32::from(header.map_id) != world.map_id()
                    || header.instance_id != world.instance_id()
                    || header.position != world.position()
                {
                    return None;
                }
            }
            let request = projection::request(self, player, &header, now)?;
            Some(PreparedPlayerSave {
                receipt: SavedPlayerReceipt {
                    expected: request.committed_groups_like_cpp(),
                    tutorials: request.tutorials.clone(),
                    handle: Some(handle),
                    owner: Some(player.capture_save_acknowledgement_like_cpp()),
                },
                request,
                header,
            })
        })?
    }

    pub(in crate::session) fn player_save_header_from_owner_like_cpp(
        &self,
        player: &wow_entities::Player,
        residence: wow_map::PlayerResidenceLikeCpp,
    ) -> PlayerSaveToDbSnapshotLikeCpp {
        let teleport = &player.gameplay_state().teleport;
        let destination = (teleport.recovery != wow_entities::PlayerTransferRecovery::Terminal)
            .then(|| {
                self.pending_teleport
                    .map(|(map, position)| (u16::try_from(map).unwrap_or(u16::MAX), position))
                    .or_else(|| {
                        teleport
                            .near_pending
                            .then_some(teleport.near_destination)
                            .flatten()
                    })
            })
            .flatten();
        let (map_id, instance_id, position) = if let Some((map_id, position)) = destination {
            (map_id, 0, position)
        } else {
            // Player.cpp:19480-19514 reads the Player's location. ResetMap
            // (Object.cpp:1814) retains map/instance even while detached.
            (
                player.unit().world().map_id() as u16,
                player.unit().world().instance_id(),
                player.unit().world().position(),
            )
        };
        let unit = player.unit();
        let max_health = unit.data().max_health.clamp(1, u64::from(u32::MAX)) as u32;
        let health = match residence {
            wow_map::PlayerResidenceLikeCpp::Active(_) => {
                let health = unit.data().health.min(u64::from(u32::MAX)) as u32;
                if unit.is_alive() && health > 0 {
                    health
                } else {
                    0
                }
            }
            wow_map::PlayerResidenceLikeCpp::Detached => {
                unit.data().health.min(u64::from(max_health)) as u32
            }
        };
        PlayerSaveToDbSnapshotLikeCpp {
            guid: player.guid(),
            map_id,
            instance_id,
            position,
            level: unit.data().level as u8, // C++ Unit::GetLevel (Unit.h:733).
            xp: player.active_data().xp.max(0) as u32,
            money: player.money(),
            health,
            max_health,
            powers: loaded_character_power_snapshot_like_cpp(unit.data().power),
        }
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
        let expected = self.expected;
        let groups = PlayerCharacterCommittedGroupsLikeCpp {
            player_spells: expected.player_spells && committed.player_spells,
            fallback_player_spells: expected.fallback_player_spells
                && committed.fallback_player_spells,
            player_skills: expected.player_skills && committed.player_skills,
            equipment_sets: expected.equipment_sets && committed.equipment_sets,
            tutorials_changed: expected.tutorials_changed && committed.tutorials_changed,
            tutorials_insert: expected.tutorials_insert && committed.tutorials_insert,
            reputation: expected.reputation && committed.reputation,
        };
        #[cfg(test)]
        if self.handle.is_none() {
            session.mark_current_player_save_to_db_committed_like_cpp(&groups);
            return;
        }
        let Some(handle) = self.handle else {
            return;
        };
        if session.player_handle_like_cpp != Some(handle)
            || session.player_guid() != Some(handle.guid())
        {
            return;
        }
        let Some(owner) = self.owner else {
            return;
        };
        let Some(manager) = session.canonical_map_manager.as_ref() else {
            return;
        };
        let acknowledged = manager
            .lock()
            .ok()
            .and_then(|mut manager| {
                manager.with_player_mut_like_cpp(handle, |player| {
                    player.acknowledge_saved_projection_like_cpp(
                        owner,
                        wow_entities::PlayerSavedGroupsLikeCpp {
                            spells: groups.player_spells,
                            fallback_spells: groups.fallback_player_spells,
                            skills: groups.player_skills,
                            equipment: groups.equipment_sets,
                            reputations: groups.reputation,
                        },
                    );
                })
            })
            .is_some();
        if !acknowledged {
            return;
        }
        // Session is exclusively borrowed through the save. Keep an explicit value
        // check for its account-owned tutorials; do not extend the Player receipt to them.
        if let Some(saved) = self.tutorials {
            if groups.tutorials_insert {
                session.tutorials_loaded_from_db_like_cpp = true;
            }
            if groups.tutorials_changed && session.tutorials_like_cpp == saved.tutorials {
                session.tutorials_changed_like_cpp = false;
            }
        }
        // Derived registry publication occurs after releasing the canonical owner guard.
        if groups.player_spells || groups.player_skills {
            session.sync_player_registry_state_like_cpp();
        }
    }
}
