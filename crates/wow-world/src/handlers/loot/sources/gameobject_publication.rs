//! Shared gameobject state synchronization and visible publication commands.
use super::*;

impl WorldSession {
    /// Mirrors the small gathering-node state subset that C++ keeps on the
    /// shared GameObject before asking this session to recompute its visible
    /// GameObject dynamic-flag deltas.
    pub(crate) fn handle_sync_gathering_node_gameobject_state_and_refresh_like_cpp(
        &mut self,
        command: SyncGatheringNodeGameobjectStateAndRefreshLikeCppCommand,
    ) {
        if self.state() != SessionState::LoggedIn {
            return;
        }
        if command.map_id != self.player_map_id_like_cpp() {
            return;
        }
        let current_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        if command.instance_id != current_instance_id {
            return;
        }
        if u32::from(command.go_type) != GAMEOBJECT_TYPE_GATHERING_NODE {
            return;
        }
        let loot_state = match command.loot_state {
            Some(0) => Some(LootState::NotReady),
            Some(1) => Some(LootState::Ready),
            Some(2) => Some(LootState::Activated),
            Some(3) => Some(LootState::JustDeactivated),
            Some(_) => return,
            None => None,
        };
        let go_state = match command.go_state {
            Some(0) => Some(GoState::Active),
            Some(1) => Some(GoState::Ready),
            Some(2) => Some(GoState::Destroyed),
            Some(24) => Some(GoState::TransportActive),
            Some(25) => Some(GoState::TransportStopped),
            Some(_) => return,
            None => None,
        };

        {
            let state = self
                .represented_gameobject_use_states
                .entry(command.gameobject_guid)
                .or_default();
            state.map_id = Some(command.map_id);
            state.go_type = Some(command.go_type);
            state.loot_state = loot_state;
            state.loot_state_unit_guid = command.loot_state_unit_guid;
            state.go_state = go_state;
            state.dynamic_flags = command.dynamic_flags;
            state.gathering_node_loot_id = command.gathering_node_loot_id;
            state.personal_loot_uses = command.personal_loot_uses;
            state.linked_trap_entry = command.linked_trap_entry;
            state.linked_trap_guid = command.linked_trap_guid;
        }

        let _ = self.update_visible_gameobjects_or_spell_clicks_like_cpp();
    }

    /// Mirrors the small chest state subset that C++ keeps on the shared
    /// GameObject before asking this session to recompute visible GameObject
    /// dynamic-flag deltas.
    pub(crate) fn handle_sync_chest_gameobject_state_and_refresh_like_cpp(
        &mut self,
        command: SyncChestGameobjectStateAndRefreshLikeCppCommand,
    ) {
        if self.state() != SessionState::LoggedIn {
            return;
        }
        if command.map_id != self.player_map_id_like_cpp() {
            return;
        }
        let current_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        if command.instance_id != current_instance_id {
            return;
        }
        if u32::from(command.go_type) != GAMEOBJECT_TYPE_CHEST {
            return;
        }
        let loot_state = match command.loot_state {
            Some(0) => Some(LootState::NotReady),
            Some(1) => Some(LootState::Ready),
            Some(2) => Some(LootState::Activated),
            Some(3) => Some(LootState::JustDeactivated),
            Some(_) => return,
            None => None,
        };

        {
            let state = self
                .represented_gameobject_use_states
                .entry(command.gameobject_guid)
                .or_default();
            state.map_id = Some(command.map_id);
            state.go_type = Some(command.go_type);
            state.loot_state = loot_state;
            state.loot_state_unit_guid = command.loot_state_unit_guid;
            state.chest_loot_source = Some(GameObjectLootSource {
                loot_id: command.chest_loot_id,
                use_group_loot_rules: false,
                dungeon_encounter_id: 0,
                personal_loot_id: command.chest_personal_loot_id,
                push_loot_id: command.chest_push_loot_id,
                triggered_event_id: 0,
                linked_trap_entry: command.linked_trap_entry.unwrap_or_default(),
                chest_restock_time_secs: command.chest_restock_time_secs,
                chest_consumable: command.chest_consumable,
                chest_quest_id: command.chest_quest_id,
            });
            state.chest_restock_time_secs = Some(command.chest_restock_time_secs);
            state.chest_consumable = Some(command.chest_consumable);
            state.chest_personal_loot_id = Some(command.chest_personal_loot_id);
            state.linked_trap_entry = command.linked_trap_entry;
            state.linked_trap_guid = command.linked_trap_guid;
        }

        let _ = self.update_visible_gameobjects_or_spell_clicks_like_cpp();
    }

    /// Mirrors the small shared goober state subset that C++ keeps on the
    /// shared GameObject before asking this session to recompute visible
    /// GameObject dynamic-flag deltas. This intentionally does not import the
    /// cooldown/source ownership fields; the map-owned close/despawn path is a
    /// later runtime slice.
    pub(crate) fn handle_sync_goober_gameobject_state_and_refresh_like_cpp(
        &mut self,
        command: SyncGooberGameobjectStateAndRefreshLikeCppCommand,
    ) {
        if self.state() != SessionState::LoggedIn {
            return;
        }
        if command.map_id != self.player_map_id_like_cpp() {
            return;
        }
        let current_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        if command.instance_id != current_instance_id {
            return;
        }
        if u32::from(command.go_type) != GAMEOBJECT_TYPE_GOOBER {
            return;
        }
        let loot_state = match command.loot_state {
            Some(0) => Some(LootState::NotReady),
            Some(1) => Some(LootState::Ready),
            Some(2) => Some(LootState::Activated),
            Some(3) => Some(LootState::JustDeactivated),
            Some(_) => return,
            None => None,
        };
        let go_state = match command.go_state {
            Some(0) => Some(GoState::Active),
            Some(1) => Some(GoState::Ready),
            Some(2) => Some(GoState::Destroyed),
            Some(24) => Some(GoState::TransportActive),
            Some(25) => Some(GoState::TransportStopped),
            Some(_) => return,
            None => None,
        };

        {
            let state = self
                .represented_gameobject_use_states
                .entry(command.gameobject_guid)
                .or_default();
            state.map_id = Some(command.map_id);
            state.go_type = Some(command.go_type);
            state.gameobject_flags = command.gameobject_flags;
            state.loot_state = loot_state;
            state.loot_state_unit_guid = command.loot_state_unit_guid;
            state.go_state = go_state;
            state.dynamic_flags = command.dynamic_flags;
            state.linked_trap_entry = command.linked_trap_entry;
            state.linked_trap_guid = command.linked_trap_guid;
        }

        let _ = self.update_visible_gameobjects_or_spell_clicks_like_cpp();
    }


    pub(super) fn gathering_node_gameobject_state_refresh_command_like_cpp(
        &self,
        gameobject_guid: ObjectGuid,
    ) -> Option<SyncGatheringNodeGameobjectStateAndRefreshLikeCppCommand> {
        let state = self
            .represented_gameobject_use_states
            .get(&gameobject_guid)?;
        Some(SyncGatheringNodeGameobjectStateAndRefreshLikeCppCommand {
            gameobject_guid,
            map_id: self.player_map_id_like_cpp(),
            instance_id: self
                .current_canonical_player_map_key_like_cpp()
                .map(|key| key.instance_id)
                .unwrap_or(0),
            go_type: state.go_type?,
            loot_state: state.loot_state.map(|loot_state| loot_state as u8),
            loot_state_unit_guid: state.loot_state_unit_guid,
            go_state: state.go_state.map(|go_state| go_state as i8),
            dynamic_flags: state.dynamic_flags,
            gathering_node_loot_id: state.gathering_node_loot_id,
            personal_loot_uses: state.personal_loot_uses,
            linked_trap_entry: state.linked_trap_entry,
            linked_trap_guid: state.linked_trap_guid,
        })
    }

    pub(super) fn chest_gameobject_state_refresh_command_like_cpp(
        &self,
        gameobject_guid: ObjectGuid,
    ) -> Option<SyncChestGameobjectStateAndRefreshLikeCppCommand> {
        let state = self
            .represented_gameobject_use_states
            .get(&gameobject_guid)?;
        let source = state.chest_loot_source?;
        Some(SyncChestGameobjectStateAndRefreshLikeCppCommand {
            gameobject_guid,
            map_id: self.player_map_id_like_cpp(),
            instance_id: self
                .current_canonical_player_map_key_like_cpp()
                .map(|key| key.instance_id)
                .unwrap_or(0),
            go_type: state.go_type.unwrap_or(GAMEOBJECT_TYPE_CHEST as u8),
            loot_state: state.loot_state.map(|loot_state| loot_state as u8),
            loot_state_unit_guid: state.loot_state_unit_guid,
            chest_loot_id: source.loot_id,
            chest_personal_loot_id: source.personal_loot_id,
            chest_push_loot_id: source.push_loot_id,
            chest_quest_id: source.chest_quest_id,
            chest_restock_time_secs: source.chest_restock_time_secs,
            chest_consumable: source.chest_consumable,
            linked_trap_entry: state.linked_trap_entry,
            linked_trap_guid: state.linked_trap_guid,
        })
    }

    pub(super) fn goober_gameobject_state_refresh_command_like_cpp(
        &self,
        gameobject_guid: ObjectGuid,
    ) -> Option<SyncGooberGameobjectStateAndRefreshLikeCppCommand> {
        let state = self
            .represented_gameobject_use_states
            .get(&gameobject_guid)?;
        Some(SyncGooberGameobjectStateAndRefreshLikeCppCommand {
            gameobject_guid,
            map_id: self.player_map_id_like_cpp(),
            instance_id: self
                .current_canonical_player_map_key_like_cpp()
                .map(|key| key.instance_id)
                .unwrap_or(0),
            go_type: state.go_type.unwrap_or(GAMEOBJECT_TYPE_GOOBER as u8),
            gameobject_flags: state.gameobject_flags,
            loot_state: state.loot_state.map(|loot_state| loot_state as u8),
            loot_state_unit_guid: state.loot_state_unit_guid,
            go_state: state.go_state.map(|go_state| go_state as i8),
            dynamic_flags: state.dynamic_flags,
            linked_trap_entry: state.linked_trap_entry,
            linked_trap_guid: state.linked_trap_guid,
        })
    }

    pub(crate) fn queue_chest_gameobject_state_refresh_for_same_map_like_cpp(
        &self,
        gameobject_guid: ObjectGuid,
    ) -> usize {
        let Some(player_guid) = self.player_guid() else {
            return 0;
        };
        let Some(registry) = self.player_registry() else {
            return 0;
        };
        let Some(command) = self.chest_gameobject_state_refresh_command_like_cpp(gameobject_guid)
        else {
            return 0;
        };
        let current_map_id = self.player_map_id_like_cpp();
        let current_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let mut queued = 0;

        for registration in
            registry.same_map_loot_recipients(player_guid, current_map_id, current_instance_id)
        {
            if registry
                .try_send_current_command(
                    registration,
                    SessionCommand::SyncChestGameobjectStateAndRefreshLikeCpp(command.clone()),
                )
                .is_ok()
            {
                queued += 1;
            }
        }

        queued
    }

    pub(crate) fn queue_goober_gameobject_state_refresh_for_same_map_like_cpp(
        &self,
        gameobject_guid: ObjectGuid,
    ) -> usize {
        let Some(player_guid) = self.player_guid() else {
            return 0;
        };
        let Some(registry) = self.player_registry() else {
            return 0;
        };
        let Some(command) = self.goober_gameobject_state_refresh_command_like_cpp(gameobject_guid)
        else {
            return 0;
        };
        let current_map_id = self.player_map_id_like_cpp();
        let current_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let mut queued = 0;

        for registration in
            registry.same_map_loot_recipients(player_guid, current_map_id, current_instance_id)
        {
            if registry
                .try_send_current_command(
                    registration,
                    SessionCommand::SyncGooberGameobjectStateAndRefreshLikeCpp(command.clone()),
                )
                .is_ok()
            {
                queued += 1;
            }
        }

        queued
    }

    pub(crate) fn queue_visible_gameobject_packet_for_same_map_like_cpp(
        &self,
        gameobject_guid: ObjectGuid,
        packet_bytes: Vec<u8>,
    ) -> usize {
        let Some(player_guid) = self.player_guid() else {
            return 0;
        };
        let Some(registry) = self.player_registry() else {
            return 0;
        };
        let current_map_id = self.player_map_id_like_cpp();
        let current_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let mut queued = 0;

        for registration in
            registry.same_map_loot_recipients(player_guid, current_map_id, current_instance_id)
        {
            if registry
                .try_send_current_command(
                    registration,
                    SessionCommand::SendIfVisibleLikeCpp(SendIfVisibleLikeCppCommand {
                        queued_at: Instant::now(),
                        source_guid: gameobject_guid,
                        map_id: current_map_id,
                        instance_id: current_instance_id,
                        packet_bytes: packet_bytes.clone(),
                    }),
                )
                .is_ok()
            {
                queued += 1;
            }
        }

        queued
    }

    pub(in crate::handlers::loot) fn represented_creature_is_dead_for_loot_visibility_like_cpp(
        &self,
        creature_guid: ObjectGuid,
    ) -> bool {
        let (map_id, instance_id) = self.current_legacy_runtime_map_key_like_cpp();
        if let Some(manager) = self.map_manager.as_ref()
            && let Some(creature) = manager
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .find_creature(map_id, instance_id, creature_guid)
        {
            return !creature.is_alive();
        }

        let Some(map_key) =
            self.canonical_object_lookup_map_key_like_cpp(u32::from(self.player_map_id_like_cpp()))
        else {
            return false;
        };
        let Some(manager) = self.canonical_map_manager.as_ref() else {
            return false;
        };
        let Ok(manager) = manager.lock() else {
            return false;
        };
        manager
            .find_map(map_key.map_id, map_key.instance_id)
            .and_then(|map| {
                map.map()
                    .creature_transform_vitals_snapshot_like_cpp(creature_guid)
            })
            .is_some_and(|creature| !creature.is_alive)
    }

    pub(super) fn queue_gathering_node_gameobject_state_refresh_for_same_map_like_cpp(
        &self,
        gameobject_guid: ObjectGuid,
    ) -> usize {
        let Some(player_guid) = self.player_guid() else {
            return 0;
        };
        let Some(registry) = self.player_registry() else {
            return 0;
        };
        let Some(command) =
            self.gathering_node_gameobject_state_refresh_command_like_cpp(gameobject_guid)
        else {
            return 0;
        };
        let current_map_id = self.player_map_id_like_cpp();
        let current_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let mut queued = 0;

        for registration in
            registry.same_map_loot_recipients(player_guid, current_map_id, current_instance_id)
        {
            if registry
                .try_send_current_command(
                    registration,
                    SessionCommand::SyncGatheringNodeGameobjectStateAndRefreshLikeCpp(
                        command.clone(),
                    ),
                )
                .is_ok()
            {
                queued += 1;
            }
        }

        queued
    }

}
