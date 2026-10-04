use crate::InstanceState;
use wow_constants::shared::DifficultyFlags;
use wow_packet::packets::misc::{DungeonDifficultySet, RaidDifficultySet};
use wow_social::group::GroupInfo;
use wow_world_core::session::{HubMut, HubRef, InstancePlayerAccessLikeCpp};

/// Which of the three Player difficulty preferences a session transition
/// writes. C++ names them apart with one setter each
/// (`Player.h:1964-1966`); this enum keeps the same separation at the session
/// boundary instead of borrowing all three.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionDifficultyKindLikeCpp {
    Dungeon,
    Raid,
    LegacyRaid,
}

impl InstanceState {
    /// Read the three canonical Player preferences, with the original
    /// handle-less fallback whenever this domain's fixture feature is enabled.
    pub fn player_difficulty_preferences_with_access_like_cpp(
        &self,
        player: &InstancePlayerAccessLikeCpp<'_>,
    ) -> Option<(u32, u32, u32)> {
        let canonical = player.difficulty_preferences_like_cpp();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && player.owner_handle_absent_like_cpp() {
            return Some((
                self.instance_test_fixture_like_cpp
                    .represented_dungeon_difficulty_id_like_cpp,
                self.instance_test_fixture_like_cpp
                    .represented_raid_difficulty_id_like_cpp,
                self.instance_test_fixture_like_cpp
                    .represented_legacy_raid_difficulty_id_like_cpp,
            ));
        }
        canonical
    }

    /// Apply one named canonical setter, retaining this domain's original
    /// handle-less fixture fallback at the original write point.
    pub fn set_player_difficulty_with_access_like_cpp(
        &mut self,
        player: &InstancePlayerAccessLikeCpp<'_>,
        kind: SessionDifficultyKindLikeCpp,
        difficulty_id: u32,
    ) -> bool {
        #[cfg(any(test, feature = "test-fixtures"))]
        if player.owner_handle_absent_like_cpp() {
            match kind {
                SessionDifficultyKindLikeCpp::Dungeon => {
                    self.instance_test_fixture_like_cpp
                        .represented_dungeon_difficulty_id_like_cpp = difficulty_id;
                }
                SessionDifficultyKindLikeCpp::Raid => {
                    self.instance_test_fixture_like_cpp
                        .represented_raid_difficulty_id_like_cpp = difficulty_id;
                }
                SessionDifficultyKindLikeCpp::LegacyRaid => {
                    self.instance_test_fixture_like_cpp
                        .represented_legacy_raid_difficulty_id_like_cpp = difficulty_id;
                }
            }
            return true;
        }
        match kind {
            SessionDifficultyKindLikeCpp::Dungeon => {
                player.set_dungeon_difficulty_like_cpp(difficulty_id)
            }
            SessionDifficultyKindLikeCpp::Raid => {
                player.set_raid_difficulty_like_cpp(difficulty_id)
            }
            SessionDifficultyKindLikeCpp::LegacyRaid => {
                player.set_legacy_raid_difficulty_like_cpp(difficulty_id)
            }
        }
    }

    pub fn represented_toggle_difficulty_target_with_access_like_cpp(
        &self,
        player: &InstancePlayerAccessLikeCpp<'_>,
        difficulty_store: &wow_data::DifficultyStore,
    ) -> Option<u32> {
        let (dungeon, raid, _) =
            self.player_difficulty_preferences_with_access_like_cpp(player)?;
        let raid_entry = difficulty_store.get(raid);
        let entry = match raid_entry {
            Some(entry) if entry.toggle_difficulty_id != 0 => entry,
            _ => difficulty_store.get(dungeon)?,
        };
        (entry.toggle_difficulty_id != 0).then_some(u32::from(entry.toggle_difficulty_id))
    }

    pub fn represented_raid_difficulty_request_like_cpp(
        &self,
        difficulty_store: &wow_data::DifficultyStore,
        difficulty_id: i32,
        legacy: bool,
    ) -> Option<u32> {
        let difficulty_id = u32::try_from(difficulty_id).ok()?;
        let entry = difficulty_store.get(difficulty_id).copied()?;
        if entry.instance_type != wow_data::map::MAP_RAID as u8 {
            return None;
        }
        let flags = DifficultyFlags::from_bits_truncate(entry.flags);
        if !flags.contains(DifficultyFlags::CAN_SELECT) {
            return None;
        }
        (flags.contains(DifficultyFlags::LEGACY) == legacy).then_some(difficulty_id)
    }

    pub fn create_map_difficulty_context_like_cpp(
        &self,
        hub: HubRef<'_>,
        map_id: u32,
        difficulty_id: wow_map::Difficulty,
    ) -> Option<wow_map::CreateMapDifficultyContext> {
        let entries = self.create_map_db2_entries_like_cpp(hub, map_id, difficulty_id)?;

        Some(wow_map::CreateMapDifficultyContext {
            difficulty_id: entries.difficulty_id,
            has_reset_schedule: entries.has_reset_schedule(),
            is_instance_id_bound: entries.is_instance_id_bound(),
        })
    }

    pub fn represented_player_difficulty_id_for_map_entry_like_cpp(
        &self,
        hub: HubRef<'_>,
        map_id: u32,
        map_entry: wow_data::map::MapEntry,
    ) -> Option<wow_map::Difficulty> {
        let (dungeon, raid, legacy_raid) =
            self.player_difficulty_preferences_snapshot_like_cpp(hub)?;
        Some(
            (match map_entry.instance_type {
                wow_data::map::MAP_INSTANCE => dungeon,
                wow_data::map::MAP_RAID => {
                    if self.map_uses_legacy_raid_difficulty_like_cpp(hub, map_id) {
                        legacy_raid
                    } else {
                        raid
                    }
                }
                _ => 0,
            }) as wow_map::Difficulty,
        )
    }

    pub fn represented_group_difficulty_id_for_map_entry_like_cpp(
        &self,
        hub: HubRef<'_>,
        map_id: u32,
        map_entry: wow_data::map::MapEntry,
        group: &GroupInfo,
    ) -> wow_map::Difficulty {
        (match map_entry.instance_type {
            wow_data::map::MAP_INSTANCE => group.dungeon_difficulty_id,
            wow_data::map::MAP_RAID => {
                if self.map_uses_legacy_raid_difficulty_like_cpp(hub, map_id) {
                    group.legacy_raid_difficulty_id
                } else {
                    group.raid_difficulty_id
                }
            }
            _ => 0,
        }) as wow_map::Difficulty
    }

    fn map_uses_legacy_raid_difficulty_like_cpp(
        &self,
        hub: HubRef<'_>,
        map_id: u32,
    ) -> bool {
        let Some(default_difficulty) = hub.catalogs.map_difficulty_store().and_then(|store| {
            hub.catalogs
                .difficulty_store()
                .and_then(|difficulty_store| {
                    store.default_for_map_like_cpp(map_id, difficulty_store)
                })
        }) else {
            return true;
        };

        let Some(difficulty) = hub
            .catalogs
            .difficulty_store()
            .and_then(|store| store.get(u32::from(default_difficulty.difficulty_id)))
        else {
            return true;
        };

        DifficultyFlags::from_bits_truncate(difficulty.flags).contains(DifficultyFlags::LEGACY)
    }

    pub fn player_difficulty_preferences_snapshot_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<(u32, u32, u32)> {
        let player = hub.core.instance_player_access_like_cpp();
        self.player_difficulty_preferences_with_access_like_cpp(&player)
    }

    pub fn replace_player_difficulty_preferences_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        dungeon: u32,
        raid: u32,
        legacy_raid: u32,
    ) -> bool {
        let canonical = hub
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.replace_difficulty_preferences_like_cpp(dungeon, raid, legacy_raid);
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if hub.core.player_handle_like_cpp.is_none() {
            self.instance_test_fixture_like_cpp
                .represented_dungeon_difficulty_id_like_cpp = dungeon;
            self.instance_test_fixture_like_cpp
                .represented_raid_difficulty_id_like_cpp = raid;
            self.instance_test_fixture_like_cpp
                .represented_legacy_raid_difficulty_id_like_cpp = legacy_raid;
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
        hub: &mut HubMut<'_>,
        kind: SessionDifficultyKindLikeCpp,
        difficulty_id: u32,
    ) -> bool {
        let player = hub.core.instance_player_access_like_cpp();
        self.set_player_difficulty_with_access_like_cpp(&player, kind, difficulty_id)
    }

    pub fn resolved_dungeon_difficulty_id_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<u32> {
        self.player_difficulty_preferences_snapshot_like_cpp(hub)
            .map(|preferences| preferences.0)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_dungeon_difficulty_id_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> u32 {
        self.resolved_dungeon_difficulty_id_like_cpp(hub)
            .expect("test Player difficulty owner must resolve")
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_represented_dungeon_difficulty_id_for_test_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        difficulty_id: u32,
    ) {
        let _ = self.set_player_difficulty_like_cpp(
            hub,
            SessionDifficultyKindLikeCpp::Dungeon,
            difficulty_id,
        );
    }

    pub fn represented_toggle_difficulty_target_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<u32> {
        let store = hub.catalogs.difficulty_store()?;
        let player = hub.core.instance_player_access_like_cpp();
        self.represented_toggle_difficulty_target_with_access_like_cpp(&player, store)
    }

    pub fn represented_dungeon_difficulty_packet_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<DungeonDifficultySet> {
        Some(DungeonDifficultySet {
            difficulty_id: i32::try_from(self.resolved_dungeon_difficulty_id_like_cpp(hub)?)
                .unwrap_or(i32::MAX),
        })
    }

    /// #743: converge this member's difficulty preferences on its group.
    ///
    /// C++ `Group::SetDungeonDifficultyID`/`SetRaidDifficultyID`/
    /// `SetLegacyRaidDifficultyID` write every connected member's
    /// `Player::m_dungeonDifficulty` family and send the matching `*DifficultySet`
    /// inside the same operation, so no member keeps its own value while in the
    /// group. This reapplies exactly those three values and publishes only the
    /// kinds that actually changed, for a member whose notification was lost.
    pub fn reconcile_group_difficulty_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        dungeon: u32,
        raid: u32,
        legacy_raid: u32,
    ) -> bool {
        let Some((current_dungeon, current_raid, current_legacy_raid)) =
            self.player_difficulty_preferences_snapshot_like_cpp(hub.shared())
        else {
            return false;
        };
        if (current_dungeon, current_raid, current_legacy_raid) == (dungeon, raid, legacy_raid) {
            return false;
        }
        if !self.replace_player_difficulty_preferences_like_cpp(hub, dungeon, raid, legacy_raid) {
            return false;
        }
        if current_dungeon != dungeon {
            hub.core.send_packet(&DungeonDifficultySet {
                difficulty_id: i32::try_from(dungeon).unwrap_or(i32::MAX),
            });
        }
        if current_raid != raid {
            hub.core.send_packet(&RaidDifficultySet {
                difficulty_id: i32::try_from(raid).unwrap_or(i32::MAX),
                legacy: false,
            });
        }
        if current_legacy_raid != legacy_raid {
            hub.core.send_packet(&RaidDifficultySet {
                difficulty_id: i32::try_from(legacy_raid).unwrap_or(i32::MAX),
                legacy: true,
            });
        }
        true
    }
}
