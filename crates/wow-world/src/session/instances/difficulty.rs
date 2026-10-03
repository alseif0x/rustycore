//! Represented difficulty selection and its published state.
//!
//! Moved out of the Session root under #613. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;
use wow_world_instances::SessionDifficultyKindLikeCpp;

impl WorldSession {
    pub(crate) fn create_map_difficulty_context_like_cpp(
        &self,
        map_id: u32,
        difficulty_id: wow_map::Difficulty,
    ) -> Option<wow_map::CreateMapDifficultyContext> {
        let (state, hub) = crate::session::split_instances_ref(self);
        state.create_map_difficulty_context_like_cpp(hub, map_id, difficulty_id)
    }
    pub(in crate::session) fn represented_player_difficulty_id_for_map_entry_like_cpp(
        &self,
        map_id: u32,
        map_entry: wow_data::map::MapEntry,
    ) -> Option<wow_map::Difficulty> {
        let (state, hub) = crate::session::split_instances_ref(self);
        state.represented_player_difficulty_id_for_map_entry_like_cpp(hub, map_id, map_entry)
    }
    pub(in crate::session) fn represented_group_difficulty_id_for_map_entry_like_cpp(
        &self,
        map_id: u32,
        map_entry: wow_data::map::MapEntry,
        group: &GroupInfo,
    ) -> wow_map::Difficulty {
        let (state, hub) = crate::session::split_instances_ref(self);
        state.represented_group_difficulty_id_for_map_entry_like_cpp(hub, map_id, map_entry, group)
    }
    /// Set the C++ Difficulty.db2 store used by `sDifficultyStore`.
    pub fn set_difficulty_store(&mut self, store: Arc<DifficultyStore>) {
        self.core
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        self.catalogs.difficulty_store = Some(store);
    }
    pub(crate) fn player_difficulty_preferences_snapshot_like_cpp(
        &self,
    ) -> Option<(u32, u32, u32)> {
        let (state, hub) = crate::session::split_instances_ref(self);
        state.player_difficulty_preferences_snapshot_like_cpp(hub)
    }
    pub(in crate::session) fn replace_player_difficulty_preferences_like_cpp(
        &mut self,
        dungeon: u32,
        raid: u32,
        legacy_raid: u32,
    ) -> bool {
        let (state, mut hub) = crate::session::split_instances_mut(self);
        state.replace_player_difficulty_preferences_like_cpp(&mut hub, dungeon, raid, legacy_raid)
    }
    fn set_player_difficulty_like_cpp(
        &mut self,
        kind: SessionDifficultyKindLikeCpp,
        difficulty_id: u32,
    ) -> bool {
        let (state, mut hub) = crate::session::split_instances_mut(self);
        state.set_player_difficulty_like_cpp(&mut hub, kind, difficulty_id)
    }
    pub(crate) fn resolved_dungeon_difficulty_id_like_cpp(&self) -> Option<u32> {
        let (state, hub) = crate::session::split_instances_ref(self);
        state.resolved_dungeon_difficulty_id_like_cpp(hub)
    }
    pub(crate) fn resolved_raid_difficulty_id_like_cpp(&self) -> Option<u32> {
        self.player_difficulty_preferences_snapshot_like_cpp()
            .map(|preferences| preferences.1)
    }
    pub(crate) fn resolved_legacy_raid_difficulty_id_like_cpp(&self) -> Option<u32> {
        self.player_difficulty_preferences_snapshot_like_cpp()
            .map(|preferences| preferences.2)
    }
    #[cfg(test)]
    pub(crate) fn represented_raid_difficulty_id_like_cpp(&self) -> u32 {
        self.resolved_raid_difficulty_id_like_cpp()
            .expect("test Player raid difficulty owner must resolve")
    }
    #[cfg(test)]
    pub(crate) fn represented_legacy_raid_difficulty_id_like_cpp(&self) -> u32 {
        self.resolved_legacy_raid_difficulty_id_like_cpp()
            .expect("test Player legacy raid difficulty owner must resolve")
    }
    pub(crate) fn represented_toggle_difficulty_target_like_cpp(&self) -> Option<u32> {
        let (state, hub) = crate::session::split_instances_ref(self);
        state.represented_toggle_difficulty_target_like_cpp(hub)
    }
    pub(crate) fn represented_dungeon_difficulty_packet_like_cpp(
        &self,
    ) -> Option<DungeonDifficultySet> {
        let (state, hub) = crate::session::split_instances_ref(self);
        state.represented_dungeon_difficulty_packet_like_cpp(hub)
    }
    pub(crate) fn apply_group_difficulty_like_cpp(
        &mut self,
        group_guid: u64,
        difficulty_id: u32,
        kind: wow_social::group::GroupDifficultyKindLikeCpp,
    ) {
        if self.resolved_group_guid_like_cpp() != Some(group_guid) {
            return;
        }

        let session_kind = match kind {
            wow_social::group::GroupDifficultyKindLikeCpp::Dungeon => {
                SessionDifficultyKindLikeCpp::Dungeon
            }
            wow_social::group::GroupDifficultyKindLikeCpp::Raid => {
                SessionDifficultyKindLikeCpp::Raid
            }
            wow_social::group::GroupDifficultyKindLikeCpp::LegacyRaid => {
                SessionDifficultyKindLikeCpp::LegacyRaid
            }
        };
        if !self.set_player_difficulty_like_cpp(session_kind, difficulty_id) {
            return;
        }

        match kind {
            wow_social::group::GroupDifficultyKindLikeCpp::Dungeon => {
                self.send_packet(&DungeonDifficultySet {
                    difficulty_id: i32::try_from(difficulty_id).unwrap_or(i32::MAX),
                });
            }
            wow_social::group::GroupDifficultyKindLikeCpp::Raid => {
                self.send_packet(&RaidDifficultySet {
                    difficulty_id: i32::try_from(difficulty_id).unwrap_or(i32::MAX),
                    legacy: false,
                });
            }
            wow_social::group::GroupDifficultyKindLikeCpp::LegacyRaid => {
                self.send_packet(&RaidDifficultySet {
                    difficulty_id: i32::try_from(difficulty_id).unwrap_or(i32::MAX),
                    legacy: true,
                });
            }
        }
    }
    pub(in crate::session) fn reconcile_group_difficulty_like_cpp(
        &mut self,
        dungeon: u32,
        raid: u32,
        legacy_raid: u32,
    ) -> bool {
        let (state, mut hub) = crate::session::split_instances_mut(self);
        state.reconcile_group_difficulty_like_cpp(&mut hub, dungeon, raid, legacy_raid)
    }

    pub(crate) fn represented_set_difficulty_id_like_cpp(
        &mut self,
        difficulty_id: u32,
    ) -> Vec<wow_persistence::RepresentedGroupPersistenceCommandLikeCpp> {
        let Some((current_dungeon, current_raid, current_legacy_raid)) =
            self.player_difficulty_preferences_snapshot_like_cpp()
        else {
            return Vec::new();
        };
        let Some(entry) = self
            .catalogs
            .difficulty_store()
            .and_then(|store| store.get(difficulty_id))
            .copied()
        else {
            return Vec::new();
        };

        let flags = DifficultyFlags::from_bits_truncate(entry.flags);
        if !flags.contains(DifficultyFlags::CAN_SELECT) {
            return Vec::new();
        }

        if self.current_map_instanceable_like_cpp() {
            return Vec::new();
        }

        if entry.instance_type == MAP_INSTANCE_LIKE_CPP {
            if let Some(statement) = self.set_represented_group_difficulty_like_cpp(
                difficulty_id,
                wow_social::group::GroupDifficultyKindLikeCpp::Dungeon,
            ) {
                return vec![statement];
            }
            if self.resolved_group_guid_like_cpp().is_some() || difficulty_id == current_dungeon {
                return Vec::new();
            }

            if !self.set_player_difficulty_like_cpp(
                SessionDifficultyKindLikeCpp::Dungeon,
                difficulty_id,
            ) {
                return Vec::new();
            }
            self.send_packet(&DungeonDifficultySet {
                difficulty_id: i32::try_from(difficulty_id).unwrap_or(i32::MAX),
            });
            Vec::new()
        } else if entry.instance_type == MAP_RAID_LIKE_CPP {
            let legacy = flags.contains(DifficultyFlags::LEGACY);
            let kind = if legacy {
                wow_social::group::GroupDifficultyKindLikeCpp::LegacyRaid
            } else {
                wow_social::group::GroupDifficultyKindLikeCpp::Raid
            };
            if let Some(statement) =
                self.set_represented_group_difficulty_like_cpp(difficulty_id, kind)
            {
                return vec![statement];
            }
            if self.resolved_group_guid_like_cpp().is_some() {
                return Vec::new();
            }
            let current = if legacy {
                current_legacy_raid
            } else {
                current_raid
            };
            if difficulty_id == current {
                return Vec::new();
            }

            if !self.set_player_difficulty_like_cpp(
                if legacy {
                    SessionDifficultyKindLikeCpp::LegacyRaid
                } else {
                    SessionDifficultyKindLikeCpp::Raid
                },
                difficulty_id,
            ) {
                return Vec::new();
            }

            self.send_packet(&RaidDifficultySet {
                difficulty_id: i32::try_from(difficulty_id).unwrap_or(i32::MAX),
                legacy,
            });
            Vec::new()
        } else {
            Vec::new()
        }
    }
    fn set_represented_group_difficulty_like_cpp(
        &mut self,
        difficulty_id: u32,
        kind: wow_social::group::GroupDifficultyKindLikeCpp,
    ) -> Option<wow_persistence::RepresentedGroupPersistenceCommandLikeCpp> {
        let group_guid = self.resolved_group_guid_like_cpp()?;
        let player_guid = self.player_guid()?;
        let registry = self.core.directory.group_registry.as_ref()?;
        let outcome = registry
            .set_difficulty_transition_like_cpp(group_guid, player_guid, difficulty_id, kind)
            .ok()?;
        let persistence = outcome.persistence;
        let members = outcome.group.members;

        for member_guid in members {
            if member_guid == player_guid {
                self.apply_group_difficulty_like_cpp(group_guid, difficulty_id, kind);
                continue;
            }
            let Some(player_registry) = self.core.player_registry.as_ref() else {
                continue;
            };
            if let Some(member) = player_registry.group_presence(member_guid) {
                let _ = player_registry.deliver_group_state_command_like_cpp(
                    member.registration,
                    SessionCommand::ApplyGroupDifficultyLikeCpp(
                        crate::session::mailbox::ApplyGroupDifficultyLikeCppCommand {
                            group_guid,
                            difficulty_id,
                            kind,
                        },
                    ),
                );
            } else {
                // C++ `Group::SetDungeonDifficultyID` writes each connected
                // member's preference in the same operation (#743).
                player_registry.mark_group_state_reconciliation_like_cpp(member_guid);
            }
        }

        persistence
            .into_iter()
            .next()
            .map(crate::handlers::group::group_persistence_command_like_cpp)
    }
    pub(crate) fn represented_set_difficulty_reset_owner_like_cpp(
        &self,
        difficulty_id: u32,
    ) -> Option<ObjectGuid> {
        let entry = self
            .catalogs
            .difficulty_store()
            .and_then(|store| store.get(difficulty_id))
            .copied()?;

        let flags = DifficultyFlags::from_bits_truncate(entry.flags);
        if !flags.contains(DifficultyFlags::CAN_SELECT) || self.current_map_instanceable_like_cpp()
        {
            return None;
        }

        let player_guid = self.player_guid()?;
        if let Some(group_guid) = self.resolved_group_guid_like_cpp() {
            let group = self
                .core
                .directory
                .group_registry
                .as_ref()?
                .get(&group_guid)?;
            if !group.is_leader_like_cpp(player_guid) || group.is_lfg_group_like_cpp() {
                return None;
            }

            let current = if entry.instance_type == MAP_INSTANCE_LIKE_CPP {
                group.dungeon_difficulty_id
            } else if entry.instance_type == MAP_RAID_LIKE_CPP {
                if flags.contains(DifficultyFlags::LEGACY) {
                    group.legacy_raid_difficulty_id
                } else {
                    group.raid_difficulty_id
                }
            } else {
                return None;
            };

            return (current != difficulty_id).then_some(group.leader_guid);
        }

        let (dungeon, raid, legacy_raid) =
            self.player_difficulty_preferences_snapshot_like_cpp()?;
        let current = if entry.instance_type == MAP_INSTANCE_LIKE_CPP {
            dungeon
        } else if entry.instance_type == MAP_RAID_LIKE_CPP {
            if flags.contains(DifficultyFlags::LEGACY) {
                legacy_raid
            } else {
                raid
            }
        } else {
            return None;
        };

        (current != difficulty_id).then_some(player_guid)
    }
    pub fn set_map_difficulty_store(&mut self, store: Arc<MapDifficultyStore>) {
        self.catalogs.maps.difficulty_store = Some(store);
    }
    pub fn set_map_difficulty_x_condition_store(
        &mut self,
        store: Arc<MapDifficultyXConditionStore>,
    ) {
        self.catalogs.maps.difficulty_x_condition_store = Some(store);
    }
    #[allow(dead_code)]
    pub(crate) fn represented_failed_map_difficulty_x_condition_like_cpp(
        &self,
        map_difficulty_id: u32,
    ) -> Option<u32> {
        let store = self.catalogs.maps.difficulty_x_condition_store.as_ref()?;
        let player_conditions = self.catalogs.player_condition_store.as_ref()?;
        let context = self.represented_player_condition_context_like_cpp()?;
        store.failed_condition_like_cpp(map_difficulty_id, player_conditions, |condition| {
            context
                .as_context(self)
                .is_some_and(|context| is_player_meeting_condition_like_cpp(condition, &context))
        })
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/instances/difficulty/f3_shims.rs"]
mod f3_shims;
