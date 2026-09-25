// Copyright (c) 2026 alseif0x
//! Feature-gated fixture accessors and tests for the session directory.

use super::*;

impl PlayerRegistry {
    /// Clone the only non-canonical fixture value without exposing storage.
    #[cfg(any(test, feature = "test-fixtures"))]
    #[must_use]
    pub fn fixture_active_loot_rolls(
        &self,
        guid: ObjectGuid,
    ) -> Option<Vec<LootRollCommandIdentityLikeCpp>> {
        self.entries
            .get(&guid)
            .map(|entry| entry.active_loot_rolls.clone())
    }

    /// Clone one fixture entry's durable creature rail. The projection no
    /// longer carries it (#270), and a test that drains the rail is addressing
    /// the entry, not reading gameplay state.
    #[cfg(any(test, feature = "test-fixtures"))]
    #[must_use]
    pub fn fixture_durable_creature_runtime_commands_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<Arc<Mutex<DurableCreatureRuntimeCommandsLikeCpp>>> {
        self.entries
            .get(&guid)
            .map(|entry| Arc::clone(&entry.durable_creature_runtime_commands_like_cpp))
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_update(
        &self,
        guid: ObjectGuid,
        update: impl FnOnce(&mut PlayerDirectoryPlacementLikeCpp),
    ) -> bool {
        let Some(mut entry) = self.entries.get_mut(&guid) else {
            return false;
        };
        let entry = &mut *entry;
        update(&mut entry.placement);
        true
    }

    /// Remove one fixture registration without exporting its storage record.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn fixture_remove(&self, guid: ObjectGuid) -> bool {
        self.entries.remove(&guid).is_some()
    }

    /// Count connected fixture registrations without exposing iteration.
    #[cfg(any(test, feature = "test-fixtures"))]
    #[must_use]
    pub fn fixture_count(&self) -> usize {
        self.entries.len()
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn registration_for_test(
        map_id: u16,
        instance_id: u32,
        send_tx: flume::Sender<Vec<u8>>,
        command_tx: flume::Sender<SessionCommand>,
    ) -> PlayerSessionRegistrationLikeCpp {
        PlayerSessionRegistrationLikeCpp {
            identity: PlayerDirectoryIdentityLikeCpp::new("TestPlayer", 1, 0, 1, 1, 0, 2),
            placement: PlayerDirectoryPlacementLikeCpp {
                map_id,
                instance_id,
                position: Position::ZERO,
                is_in_world: true,
                level: 1,
                is_alive: true,
            },
            active_loot_rolls: Vec::new(),
            realm_send_tx: send_tx.clone(),
            send_tx,
            command_tx,
            session_phase_tx: crate::session::directory::detached_session_phase_rail_like_cpp(),
            durable_creature_runtime_commands_like_cpp: Default::default(),
            client_visible_guids_like_cpp: Default::default(),
            client_visible_transports_like_cpp: Default::default(),
            advanced_combat_logging_enabled_like_cpp: Default::default(),
            visibility_refresh_pending_like_cpp: Default::default(),
        }
    }

    /// cross-instance delivery can be filtered (Slice 4A.1b).
    /// C++ anchor: `GridNotifiersImpl.h : MessageDistDeliverer::Visit` — instance
    /// separation via `InSamePhase` + map instance ID check.
    #[test]
    fn player_directory_keeps_identity_and_placement_outside_gameplay_projection_like_cpp() {
        let (send_tx, _send_rx) = flume::bounded::<Vec<u8>>(1);
        let (command_tx, _command_rx) = flume::bounded::<SessionCommand>(1);
        let info = registration_for_test(571, 42, send_tx, command_tx);
        assert_eq!(info.placement.instance_id, 42);
        assert_eq!(info.placement.map_id, 571);

        let registry = PlayerRegistry::new();
        let alpha = ObjectGuid::create_player(1, 100);
        let beta = ObjectGuid::create_player(1, 101);
        let canonical: SharedCanonicalMapManager =
            Arc::new(Mutex::new(wow_map::MapManager::default()));
        assert!(registry.bind_canonical_map_manager(Arc::clone(&canonical)));
        {
            let mut manager = canonical.lock().unwrap();
            let map = manager.create_world_map(571, 42).map_mut();
            for guid in [alpha, beta] {
                let mut player = wow_entities::Player::new(Some(1), false);
                player.unit_mut().world_mut().object_mut().create(guid);
                player.unit_mut().world_mut().set_map(571, 42).unwrap();
                player.unit_mut().world_mut().object_mut().add_to_world();
                player.set_dungeon_difficulty_id_like_cpp(1);
                map.insert_map_object_record(
                    wow_entities::MapObjectRecord::new_player(player).unwrap(),
                )
                .unwrap();
            }
        }
        let first_alpha = registry.register_or_replace(alpha, info.clone(), Default::default());

        let social = registry
            .social_recipient_by_name("testplayer")
            .expect("case-insensitive connected-player lookup");
        assert_eq!(social.registration, first_alpha);
        assert_eq!(social.map_id, 571);
        assert_eq!(social.instance_id, 42);
        assert_eq!(social.dungeon_difficulty_id, 1);

        let mut replacement = info.clone();
        replacement.identity.player_name = "Replacement".to_string();
        let replacement_alpha =
            registry.register_or_replace(alpha, replacement, Default::default());
        assert_eq!(
            registry.send_current_packet(first_alpha, vec![1]),
            Err(PlayerDirectorySendError::StaleRegistration),
            "an older social/group decision must not reach a replacement session"
        );
        assert_eq!(
            registry.social_recipient(alpha).unwrap().registration,
            replacement_alpha
        );

        let mut beta_info = info;
        beta_info.identity.player_name = "Beta".to_string();
        registry.register_or_replace(beta, beta_info, Default::default());
        let ordered =
            registry.group_presences_in_order(&[beta, ObjectGuid::create_player(1, 999), alpha]);
        assert_eq!(
            ordered.iter().map(|member| member.guid).collect::<Vec<_>>(),
            vec![beta, alpha],
            "connected Group projections preserve authoritative member order"
        );
    }

    /// Place one canonical `Player` on an exact map instance with a known honor
    /// level, so an inspect resolver has a real owner to read.
    fn canonical_map_with_player_for_test(
        guid: ObjectGuid,
        map_id: u32,
        instance_id: u32,
        honor_level: i32,
    ) -> SharedCanonicalMapManager {
        let mut player = wow_entities::Player::new(Some(1), false);
        player.unit_mut().world_mut().object_mut().create(guid);
        player.unit_mut().world_mut().set_name("Inspected");
        player
            .unit_mut()
            .world_mut()
            .set_map(map_id, instance_id)
            .unwrap();
        player.unit_mut().world_mut().object_mut().add_to_world();
        player.set_honor_level_like_cpp(honor_level);

        let canonical: SharedCanonicalMapManager =
            Arc::new(Mutex::new(wow_map::MapManager::default()));
        canonical
            .lock()
            .unwrap()
            .create_world_map(map_id, instance_id)
            .map_mut()
            .insert_map_object_record(wow_entities::MapObjectRecord::new_player(player).unwrap())
            .unwrap();
        canonical
    }

    /// The inspect honor block comes off the target's canonical `Player`, not off
    /// a mirrored copy (#252).
    ///
    /// C++ anchor: `Player::SendInspectResult` reads the honor fields straight
    /// from the inspected `Player`'s `m_activePlayerData`/`m_playerData`; it does
    /// not consult a per-session cache of another player's state.
    #[test]
    fn inspect_honor_stats_reads_the_level_from_the_canonical_player_like_cpp() {
        let (send_tx, _send_rx) = flume::bounded::<Vec<u8>>(1);
        let (command_tx, _command_rx) = flume::bounded::<SessionCommand>(1);
        let guid = ObjectGuid::create_player(1, 700);
        let canonical = canonical_map_with_player_for_test(guid, 571, 42, 7);

        let registry = PlayerRegistry::new();
        registry.register_or_replace(
            guid,
            registration_for_test(571, 42, send_tx, command_tx),
            Default::default(),
        );

        let (.., honor_level) = registry
            .inspect_honor_stats(guid, Some(&canonical))
            .expect("canonical owner resolves");
        assert_eq!(honor_level, 7);
    }

    /// Retiring the mirror also retires its staleness window.
    ///
    /// The honor level used to be copied in at registration and refreshed only
    /// on the next registry sync, so an inspect between the two showed the old
    /// value. Reading the canonical owner makes that window unrepresentable:
    /// nothing republishes here, and the new value is still observed.
    #[test]
    fn inspect_honor_stats_sees_a_canonical_change_without_a_registry_sync_like_cpp() {
        let (send_tx, _send_rx) = flume::bounded::<Vec<u8>>(1);
        let (command_tx, _command_rx) = flume::bounded::<SessionCommand>(1);
        let guid = ObjectGuid::create_player(1, 701);
        let canonical = canonical_map_with_player_for_test(guid, 571, 42, 3);

        let registry = PlayerRegistry::new();
        registry.register_or_replace(
            guid,
            registration_for_test(571, 42, send_tx, command_tx),
            Default::default(),
        );
        assert_eq!(
            registry
                .inspect_honor_stats(guid, Some(&canonical))
                .expect("canonical owner resolves")
                .6,
            3
        );

        canonical
            .lock()
            .unwrap()
            .find_map_mut(571, 42)
            .expect("resident map instance")
            .map_mut()
            .get_typed_player_mut(guid)
            .expect("canonical player")
            .set_honor_level_like_cpp(9);

        assert_eq!(
            registry
                .inspect_honor_stats(guid, Some(&canonical))
                .expect("canonical owner resolves")
                .6,
            9,
            "inspect must read the canonical owner, not a copy taken at registration"
        );
    }

    /// An unresolvable canonical owner reports *unknown*, never a fabricated zero.
    ///
    /// This is the far-teleport window: `initiate_far_teleport_like_cpp` removes
    /// the canonical `Player` from the old map before the destination world-port
    /// response lands, while the directory registration stays. Answering zeros
    /// there would tell the client a real honor level had dropped to nothing, so
    /// the resolver returns `None` and the handler answers nothing — the branch
    /// C++ `HandleInspectHonorStats` takes when it cannot resolve the target.
    #[test]
    fn inspect_honor_stats_reports_unknown_rather_than_zero_without_an_owner_like_cpp() {
        let (send_tx, _send_rx) = flume::bounded::<Vec<u8>>(1);
        let (command_tx, _command_rx) = flume::bounded::<SessionCommand>(1);
        let guid = ObjectGuid::create_player(1, 702);

        let registry = PlayerRegistry::new();
        registry.register_or_replace(
            guid,
            registration_for_test(571, 42, send_tx, command_tx),
            Default::default(),
        );

        assert!(registry.inspect_honor_stats(guid, None).is_none());

        // Registered, but no canonical owner resident: exactly the transfer window.
        let empty: SharedCanonicalMapManager = Arc::new(Mutex::new(wow_map::MapManager::default()));
        assert!(registry.inspect_honor_stats(guid, Some(&empty)).is_none());

        assert!(registry.inspect_snapshot(guid).is_none());
    }

    /// The quest-share receiver's standings are unknown, not empty, when its
    /// canonical owner cannot be resolved (#252).
    ///
    /// The consumer turns `None` into `ReceiverEligibilityUnrepresented` instead
    /// of evaluating a standing gate against an empty set, which would report a
    /// qualifying receiver as short on reputation.
    #[test]
    fn quest_sharing_snapshot_reports_unknown_standings_without_an_owner_like_cpp() {
        let (send_tx, _send_rx) = flume::bounded::<Vec<u8>>(1);
        let (command_tx, _command_rx) = flume::bounded::<SessionCommand>(1);
        let guid = ObjectGuid::create_player(1, 703);

        let registry = PlayerRegistry::new();
        registry.register_or_replace(
            guid,
            registration_for_test(571, 42, send_tx, command_tx),
            Default::default(),
        );

        let empty: SharedCanonicalMapManager = Arc::new(Mutex::new(wow_map::MapManager::default()));
        assert!(
            registry
                .quest_sharing_snapshot(guid, Some(&empty))
                .is_none(),
            "an absent canonical owner must make the gameplay result unknown"
        );
    }
}
