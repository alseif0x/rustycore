use crate::InstanceState;
#[cfg(any(test, feature = "test-fixtures"))]
use crate::constants::HOUR_SECS_LIKE_CPP;
use wow_world_core::session::{connection_identity::unix_now, HubMut, HubRef};

impl InstanceState {
    pub fn check_instance_count_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        instance_id: u32,
    ) -> bool {
        let now_secs = u64::try_from(unix_now()).unwrap_or(0);
        self.check_instance_count_at_like_cpp(hub, instance_id, now_secs)
    }

    pub fn check_instance_count_probe_like_cpp(
        &self,
        hub: HubRef<'_>,
        instance_id: u32,
    ) -> bool {
        let now_secs = u64::try_from(unix_now()).unwrap_or(0);
        if let Some(result) = hub.core.with_owned_player_like_cpp(|player| {
            player.check_instance_count_probe_like_cpp(
                instance_id,
                now_secs,
                hub.core.realm_policy.max_instances_per_hour_like_cpp,
            )
        }) {
            return result;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if hub.core.player_handle_like_cpp.is_none() {
            return self
                .instance_reset_times_snapshot_like_cpp(hub)
                .is_some_and(|times| {
                    times
                        .values()
                        .filter(|release_time| **release_time > now_secs)
                        .count()
                        < hub.core.realm_policy.max_instances_per_hour_like_cpp as usize
                        || times
                            .get(&instance_id)
                            .is_some_and(|release_time| *release_time > now_secs)
                });
        }
        false
    }

    pub fn check_instance_count_at_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        instance_id: u32,
        now_secs: u64,
    ) -> bool {
        if let Some(result) = hub.core.with_owned_player_mut_like_cpp(|player| {
            player.check_instance_count_like_cpp(
                instance_id,
                now_secs,
                hub.core.realm_policy.max_instances_per_hour_like_cpp,
            )
        }) {
            return result;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if hub.core.player_handle_like_cpp.is_none() {
            self.prune_expired_instance_reset_times_like_cpp(hub, now_secs);
            return self
                .instance_reset_times_snapshot_like_cpp(hub.shared())
                .is_some_and(|times| {
                    times.len() < hub.core.realm_policy.max_instances_per_hour_like_cpp as usize
                        || times.contains_key(&instance_id)
                });
        }
        false
    }

    pub fn add_instance_enter_time_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        instance_id: u32,
        enter_time: u64,
    ) {
        if hub
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.add_instance_enter_time_like_cpp(instance_id, enter_time);
            })
            .is_some()
        {
            return;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if hub.core.player_handle_like_cpp.is_none() {
            self.represented_instance_reset_times_like_cpp
                .entry(instance_id)
                .or_insert(enter_time.saturating_add(HOUR_SECS_LIKE_CPP));
        }
    }

    pub fn instance_reset_times_snapshot_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<std::collections::BTreeMap<u32, u64>> {
        let canonical = hub
            .core
            .with_owned_player_like_cpp(|player| player.instance_reset_times_like_cpp().clone());
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && hub.core.player_handle_like_cpp.is_none() {
            return Some(self.represented_instance_reset_times_like_cpp.clone());
        }
        canonical
    }

    pub fn replace_instance_reset_times_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        rows: impl IntoIterator<Item = (u32, u64)>,
    ) -> bool {
        let rows: Vec<_> = rows.into_iter().collect();
        let canonical = hub.core.with_owned_player_mut_like_cpp(|player| {
            player.replace_instance_reset_times_like_cpp(rows.iter().copied());
        });
        if canonical.is_some() {
            return true;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if hub.core.player_handle_like_cpp.is_none() {
            self.represented_instance_reset_times_like_cpp.clear();
            for (instance_id, release_time) in rows {
                self.represented_instance_reset_times_like_cpp
                    .entry(instance_id)
                    .or_insert(release_time);
            }
            return true;
        }
        false
    }

    /// C++ `Player::GetRecentInstanceId`.
    pub fn resolved_player_recent_instance_id_like_cpp(
        &self,
        hub: HubRef<'_>,
        map_id: u32,
    ) -> Option<u32> {
        let canonical = hub.core.with_owned_player_like_cpp(|player| {
            player
                .gameplay_state()
                .recent_instances
                .get(&map_id)
                .copied()
                .unwrap_or(0)
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && hub.core.player_handle_like_cpp.is_none() {
            return Some(
                self.instance_test_fixture_like_cpp
                    .represented_player_recent_instances_like_cpp
                    .get(&map_id)
                    .copied()
                    .unwrap_or(0),
            );
        }
        canonical
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_player_recent_instance_id_like_cpp(
        &self,
        hub: HubRef<'_>,
        map_id: u32,
    ) -> u32 {
        self.resolved_player_recent_instance_id_like_cpp(hub, map_id)
            .expect("test Player recent-instance owner must resolve")
    }

    /// C++ `Player::SetRecentInstance`.
    pub fn set_represented_player_recent_instance_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        map_id: u32,
        instance_id: u32,
    ) -> bool {
        let canonical = hub
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_recent_instance_like_cpp(map_id, instance_id);
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if hub.core.player_handle_like_cpp.is_none() {
            self.instance_test_fixture_like_cpp
                .represented_player_recent_instances_like_cpp
                .insert(map_id, instance_id);
            return true;
        }
        canonical
    }

    pub fn forget_represented_player_recent_instance_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        map_id: u32,
    ) -> bool {
        let canonical = hub.core.with_owned_player_mut_like_cpp(|player| {
            player.forget_recent_instance_like_cpp(map_id)
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && hub.core.player_handle_like_cpp.is_none() {
            return self
                .instance_test_fixture_like_cpp
                .represented_player_recent_instances_like_cpp
                .remove(&map_id)
                .is_some();
        }
        canonical.unwrap_or(false)
    }

    pub fn current_map_instanceable_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> bool {
        let map_id = u32::from(hub.core.player_map_id_like_cpp());
        hub.catalogs
            .map_store()
            .and_then(|store| store.get(map_id))
            .is_some_and(|entry| {
                matches!(
                    entry.instance_type,
                    wow_data::map::MAP_INSTANCE
                        | wow_data::map::MAP_RAID
                        | wow_data::map::MAP_BATTLEGROUND
                        | wow_data::map::MAP_ARENA
                        | wow_data::map::MAP_SCENARIO
                )
            })
    }
}
