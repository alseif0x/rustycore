//! Represented difficulty selection and its published state.
//!
//! Moved out of the Session root under #613. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

/// Which of the three Player difficulty preferences a session transition
/// writes. C++ names them apart with one setter each
/// (`Player.h:1964-1966`); this enum keeps the same separation at the session
/// boundary instead of borrowing all three.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::session) enum SessionDifficultyKindLikeCpp {
    Dungeon,
    Raid,
    LegacyRaid,
}

impl WorldSession {
    pub(crate) fn create_map_difficulty_context_like_cpp(
        &self,
        map_id: u32,
        difficulty_id: wow_map::Difficulty,
    ) -> Option<wow_map::CreateMapDifficultyContext> {
        let entries = self.create_map_db2_entries_like_cpp(map_id, difficulty_id)?;

        Some(wow_map::CreateMapDifficultyContext {
            difficulty_id: entries.difficulty_id,
            has_reset_schedule: entries.has_reset_schedule(),
            is_instance_id_bound: entries.is_instance_id_bound(),
        })
    }
    pub(in crate::session) fn represented_player_difficulty_id_for_map_entry_like_cpp(
        &self,
        map_id: u32,
        map_entry: wow_data::map::MapEntry,
    ) -> Option<wow_map::Difficulty> {
        let (dungeon, raid, legacy_raid) =
            self.player_difficulty_preferences_snapshot_like_cpp()?;
        Some(
            (match map_entry.instance_type {
                wow_data::map::MAP_INSTANCE => dungeon,
                wow_data::map::MAP_RAID => {
                    if self.map_uses_legacy_raid_difficulty_like_cpp(map_id) {
                        legacy_raid
                    } else {
                        raid
                    }
                }
                _ => 0,
            }) as wow_map::Difficulty,
        )
    }
    pub(in crate::session) fn represented_group_difficulty_id_for_map_entry_like_cpp(
        &self,
        map_id: u32,
        map_entry: wow_data::map::MapEntry,
        group: &GroupInfo,
    ) -> wow_map::Difficulty {
        (match map_entry.instance_type {
            wow_data::map::MAP_INSTANCE => group.dungeon_difficulty_id,
            wow_data::map::MAP_RAID => {
                if self.map_uses_legacy_raid_difficulty_like_cpp(map_id) {
                    group.legacy_raid_difficulty_id
                } else {
                    group.raid_difficulty_id
                }
            }
            _ => 0,
        }) as wow_map::Difficulty
    }
    fn map_uses_legacy_raid_difficulty_like_cpp(&self, map_id: u32) -> bool {
        let Some(default_difficulty) = self.map_difficulty_store().and_then(|store| {
            self.difficulty_store().and_then(|difficulty_store| {
                store.default_for_map_like_cpp(map_id, difficulty_store)
            })
        }) else {
            return true;
        };

        let Some(difficulty) = self
            .difficulty_store()
            .and_then(|store| store.get(u32::from(default_difficulty.difficulty_id)))
        else {
            return true;
        };

        DifficultyFlags::from_bits_truncate(difficulty.flags).contains(DifficultyFlags::LEGACY)
    }
    /// Set the C++ Difficulty.db2 store used by `sDifficultyStore`.
    pub fn set_difficulty_store(&mut self, store: Arc<DifficultyStore>) {
        self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        self.difficulty_store = Some(store);
    }
    pub(crate) fn difficulty_store(&self) -> Option<&Arc<DifficultyStore>> {
        self.difficulty_store.as_ref()
    }
    pub(crate) fn player_difficulty_preferences_snapshot_like_cpp(
        &self,
    ) -> Option<(u32, u32, u32)> {
        let canonical =
            self.with_owned_player_like_cpp(|player| player.difficulty_preferences_like_cpp());
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some((
                self.represented_dungeon_difficulty_id_like_cpp,
                self.represented_raid_difficulty_id_like_cpp,
                self.represented_legacy_raid_difficulty_id_like_cpp,
            ));
        }
        canonical
    }
    pub(in crate::session) fn replace_player_difficulty_preferences_like_cpp(
        &mut self,
        dungeon: u32,
        raid: u32,
        legacy_raid: u32,
    ) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.replace_difficulty_preferences_like_cpp(dungeon, raid, legacy_raid);
            })
            .is_some();
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            self.represented_dungeon_difficulty_id_like_cpp = dungeon;
            self.represented_raid_difficulty_id_like_cpp = raid;
            self.represented_legacy_raid_difficulty_id_like_cpp = legacy_raid;
            return true;
        }
        canonical
    }
    /// Apply one named canonical difficulty setter, or the handle-less test
    /// mirror that stands in for it. C++ writes these through
    /// `Player::SetDungeonDifficultyID` and its two siblings
    /// (`Player.h:1964-1966`), never through a borrowed field.
    fn set_player_difficulty_like_cpp(
        &mut self,
        kind: SessionDifficultyKindLikeCpp,
        difficulty_id: u32,
    ) -> bool {
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            match kind {
                SessionDifficultyKindLikeCpp::Dungeon => {
                    self.represented_dungeon_difficulty_id_like_cpp = difficulty_id;
                }
                SessionDifficultyKindLikeCpp::Raid => {
                    self.represented_raid_difficulty_id_like_cpp = difficulty_id;
                }
                SessionDifficultyKindLikeCpp::LegacyRaid => {
                    self.represented_legacy_raid_difficulty_id_like_cpp = difficulty_id;
                }
            }
            return true;
        }
        self.with_owned_player_mut_like_cpp(|player| match kind {
            SessionDifficultyKindLikeCpp::Dungeon => {
                player.set_dungeon_difficulty_id_like_cpp(difficulty_id);
            }
            SessionDifficultyKindLikeCpp::Raid => {
                player.set_raid_difficulty_id_like_cpp(difficulty_id);
            }
            SessionDifficultyKindLikeCpp::LegacyRaid => {
                player.set_legacy_raid_difficulty_id_like_cpp(difficulty_id);
            }
        })
        .is_some()
    }
    pub(crate) fn resolved_dungeon_difficulty_id_like_cpp(&self) -> Option<u32> {
        self.player_difficulty_preferences_snapshot_like_cpp()
            .map(|preferences| preferences.0)
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
    pub(crate) fn represented_dungeon_difficulty_id_like_cpp(&self) -> u32 {
        self.resolved_dungeon_difficulty_id_like_cpp()
            .expect("test Player difficulty owner must resolve")
    }
    #[cfg(test)]
    pub(crate) fn set_represented_dungeon_difficulty_id_for_test_like_cpp(
        &mut self,
        difficulty_id: u32,
    ) {
        let _ = self
            .set_player_difficulty_like_cpp(SessionDifficultyKindLikeCpp::Dungeon, difficulty_id);
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
        let store = self.difficulty_store()?;
        let (dungeon, raid, _) = self.player_difficulty_preferences_snapshot_like_cpp()?;
        let raid_entry = store.get(raid);
        let entry = match raid_entry {
            Some(entry) if entry.toggle_difficulty_id != 0 => entry,
            _ => store.get(dungeon)?,
        };

        (entry.toggle_difficulty_id != 0).then_some(u32::from(entry.toggle_difficulty_id))
    }
    pub(crate) fn represented_dungeon_difficulty_packet_like_cpp(
        &self,
    ) -> Option<DungeonDifficultySet> {
        Some(DungeonDifficultySet {
            difficulty_id: i32::try_from(self.resolved_dungeon_difficulty_id_like_cpp()?)
                .unwrap_or(i32::MAX),
        })
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
    /// #743: converge this member's difficulty preferences on its group.
    ///
    /// C++ `Group::SetDungeonDifficultyID`/`SetRaidDifficultyID`/
    /// `SetLegacyRaidDifficultyID` write every connected member's
    /// `Player::m_dungeonDifficulty` family and send the matching `*DifficultySet`
    /// inside the same operation, so no member keeps its own value while in the
    /// group. This reapplies exactly those three values and publishes only the
    /// kinds that actually changed, for a member whose notification was lost.
    pub(in crate::session) fn reconcile_group_difficulty_like_cpp(
        &mut self,
        dungeon: u32,
        raid: u32,
        legacy_raid: u32,
    ) -> bool {
        let Some((current_dungeon, current_raid, current_legacy_raid)) =
            self.player_difficulty_preferences_snapshot_like_cpp()
        else {
            return false;
        };
        if (current_dungeon, current_raid, current_legacy_raid) == (dungeon, raid, legacy_raid) {
            return false;
        }
        if !self.replace_player_difficulty_preferences_like_cpp(dungeon, raid, legacy_raid) {
            return false;
        }
        if current_dungeon != dungeon {
            self.send_packet(&DungeonDifficultySet {
                difficulty_id: i32::try_from(dungeon).unwrap_or(i32::MAX),
            });
        }
        if current_raid != raid {
            self.send_packet(&RaidDifficultySet {
                difficulty_id: i32::try_from(raid).unwrap_or(i32::MAX),
                legacy: false,
            });
        }
        if current_legacy_raid != legacy_raid {
            self.send_packet(&RaidDifficultySet {
                difficulty_id: i32::try_from(legacy_raid).unwrap_or(i32::MAX),
                legacy: true,
            });
        }
        true
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
        let registry = self.group_registry.as_ref()?;
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
            let Some(player_registry) = self.player_registry.as_ref() else {
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
            let group = self.group_registry.as_ref()?.get(&group_guid)?;
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
        self.maps.difficulty_store = Some(store);
    }
    pub fn set_map_difficulty_x_condition_store(
        &mut self,
        store: Arc<MapDifficultyXConditionStore>,
    ) {
        self.maps.difficulty_x_condition_store = Some(store);
    }
    pub(crate) fn map_difficulty_store(&self) -> Option<&Arc<MapDifficultyStore>> {
        self.maps.difficulty_store.as_ref()
    }
    pub(crate) fn current_map_difficulty_id_like_cpp(&self) -> u8 {
        if let Some(difficulty_id) = self.current_canonical_player_map_difficulty_id_like_cpp() {
            return difficulty_id;
        }
        let map_id = u32::from(self.player_map_id_like_cpp());
        self.canonical_map_manager
            .as_ref()
            .and_then(|manager| manager.lock().ok())
            .and_then(|manager| {
                manager
                    .find_map(map_id, 0)
                    .map(|managed| managed.map().spawn_mode())
            })
            .unwrap_or(0)
    }
    /// C++ `Map::GetDifficultyID` for the map that actually owns this Player.
    ///
    /// The same map id can have multiple live instances. Do not infer spell
    /// metadata from the instance-zero map when the canonical player belongs
    /// to a difficulty-specific `ManagedMap`.
    pub(crate) fn current_canonical_player_map_difficulty_id_like_cpp(&self) -> Option<u8> {
        let player_guid = self.player_guid()?;
        let map_id = u32::from(self.player_map_id_like_cpp());
        let manager = self.canonical_map_manager.as_ref()?.lock().ok()?;
        let mut difficulty_id = None;
        manager.do_for_all_maps_with_map_id(map_id, |managed| {
            if difficulty_id.is_none() && managed.map().get_typed_player(player_guid).is_some() {
                difficulty_id = Some(managed.difficulty());
            }
        });
        difficulty_id
    }
    #[allow(dead_code)]
    pub(crate) fn represented_failed_map_difficulty_x_condition_like_cpp(
        &self,
        map_difficulty_id: u32,
    ) -> Option<u32> {
        let store = self.maps.difficulty_x_condition_store.as_ref()?;
        let player_conditions = self.player_condition_store.as_ref()?;
        let context = self.represented_player_condition_context_like_cpp()?;
        store.failed_condition_like_cpp(map_difficulty_id, player_conditions, |condition| {
            context
                .as_context(self)
                .is_some_and(|context| is_player_meeting_condition_like_cpp(condition, &context))
        })
    }
}
