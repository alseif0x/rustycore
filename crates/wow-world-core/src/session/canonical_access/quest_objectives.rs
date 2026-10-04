// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Narrow canonical Player access for quest-objective progress operations.

use crate::session::state::SessionCore;
use wow_core::ObjectGuid;

/// Borrowed access to the canonical quest gameplay state used by objective progress.
pub struct QuestObjectiveAccessLikeCpp<'a> {
    core: &'a SessionCore,
}

impl SessionCore {
    /// Build quest-objective access without resolving or copying the Player.
    pub fn quest_objective_access_like_cpp(&self) -> QuestObjectiveAccessLikeCpp<'_> {
        QuestObjectiveAccessLikeCpp { core: self }
    }
}

impl QuestObjectiveAccessLikeCpp<'_> {
    /// Snapshot the visible GUID set in its existing iteration order for quest refresh.
    pub fn visible_guids_snapshot_like_cpp(&self) -> Vec<ObjectGuid> {
        self.core
            .client_visible_guids_like_cpp
            .snapshot_like_cpp()
            .into_iter()
            .collect()
    }

    /// Resolve one currently visible gameobject through the canonical map owner.
    pub fn canonical_gameobject_entry_like_cpp(&self, guid: ObjectGuid) -> Option<u32> {
        self.core
            .canonical_gameobject_access_like_cpp(guid)
            .map(|access| access.entry)
    }

    /// Read one selected spell-click creature while its canonical map guard is held.
    pub fn with_spell_click_creature_like_cpp<R>(
        &self,
        guid: ObjectGuid,
        read: impl FnOnce(&wow_entities::Creature, Option<ObjectGuid>) -> R,
    ) -> Option<R> {
        if guid.is_empty() || !guid.is_any_type_creature() {
            return None;
        }
        let manager = self.core.canonical_map_manager.as_ref()?;
        let Ok(manager) = manager.lock() else {
            return None;
        };
        let map = manager.find_map(u32::from(self.core.player_map_id_like_cpp()), 0)?;
        map.map().with_creature_or_pet_like_cpp(guid, read)
    }

    /// Resolve the owner's map only when the publication phase needs it.
    pub fn player_map_id_like_cpp(&self) -> u16 {
        self.core.player_map_id_like_cpp()
    }

    /// Resolve GM presentation using the original World fixture input when enabled.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn player_is_game_master_like_cpp(&self, fixture: &bool) -> Option<bool> {
        self.core.player_is_game_master_with_fixture_like_cpp(fixture)
    }

    #[cfg(not(any(test, feature = "test-fixtures")))]
    pub fn player_is_game_master_like_cpp(&self) -> Option<bool> {
        self.core.player_is_game_master_canonical_like_cpp()
    }

    pub fn player_guid_like_cpp(&self) -> Option<ObjectGuid> {
        self.core.player_guid()
    }

    pub fn account_id_like_cpp(&self) -> u32 {
        self.core.account_id
    }

    /// Clear canonical quest-status completeness before objective processing.
    pub fn mark_quest_status_authority_incomplete_like_cpp(&self) -> Option<()> {
        self.core.mutate_canonical_player_like_cpp(|player| {
            player
                .gameplay_state_mut()
                .quests
                .set_status_authority_complete_like_cpp(false);
        })
    }

    /// Invalidate the derived spell-hit authority after quest status changes.
    pub fn invalidate_spell_hit_aura_authority_like_cpp(&self) {
        self.core
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
    }

    /// Read the current strict canonical quest gameplay state at this call site.
    pub fn player_quest_gameplay_snapshot_like_cpp(
        &self,
    ) -> Option<wow_entities::PlayerQuestGameplayState> {
        self.core
            .with_owned_player_like_cpp(|player| player.gameplay_state().quests.clone())
    }

    /// Whether this session currently has no represented Player handle.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn owner_handle_absent_like_cpp(&self) -> bool {
        self.core.player_handle_like_cpp.is_none()
    }

    /// Update one canonical objective count using the existing represented rules.
    pub fn update_objective_count_like_cpp(
        &self,
        quest_id: u32,
        objective_index: usize,
        required: i32,
        add_count: i32,
    ) -> Option<Option<i32>> {
        self.core.mutate_canonical_player_like_cpp(|player| {
            let status = player
                .gameplay_state_mut()
                .quests
                .status_mut_like_cpp(quest_id)?;
            if status.objective_counts.len() <= objective_index {
                status.objective_counts.resize(objective_index + 1, 0);
            }
            if add_count >= 0 && status.objective_counts[objective_index] >= required {
                return None;
            }
            status.objective_counts[objective_index] = status.objective_counts[objective_index]
                .saturating_add(add_count)
                .clamp(0, required);
            Some(status.objective_counts[objective_index])
        })
    }

    /// Set one canonical storing-flag objective and return its before/after state.
    pub fn update_storing_flag_like_cpp(
        &self,
        quest_id: u32,
        objective_index: usize,
        add_count: i32,
    ) -> Option<Option<(bool, bool)>> {
        self.core.mutate_canonical_player_like_cpp(|player| {
            let status = player
                .gameplay_state_mut()
                .quests
                .status_mut_like_cpp(quest_id)?;
            if status.objective_counts.len() <= objective_index {
                status.objective_counts.resize(objective_index + 1, 0);
            }
            let before = status.objective_counts[objective_index] != 0;
            status.objective_counts[objective_index] = i32::from(add_count > 0);
            Some((before, status.objective_counts[objective_index] != 0))
        })
    }

    /// Revert one canonical completed quest when its threshold objective regresses.
    pub fn mark_quest_incomplete_if_complete_like_cpp(
        &self,
        quest_id: u32,
        complete_status: u8,
        incomplete_status: u8,
    ) -> Option<bool> {
        self.core.mutate_canonical_player_like_cpp(|player| {
            let Some(status) = player
                .gameplay_state_mut()
                .quests
                .status_mut_like_cpp(quest_id)
            else {
                return false;
            };
            if status.status != complete_status {
                return false;
            }
            status.status = incomplete_status;
            true
        })
    }

    /// Complete one canonical quest status when it is still incomplete.
    pub fn complete_quest_status_like_cpp(
        &self,
        quest_id: u32,
        incomplete_status: u8,
        complete_status: u8,
    ) -> Option<Option<u8>> {
        self.core.mutate_canonical_player_like_cpp(|player| {
            let status = player
                .gameplay_state_mut()
                .quests
                .status_mut_like_cpp(quest_id)?;
            (status.status == incomplete_status).then(|| {
                let old_status = status.status;
                status.status = complete_status;
                old_status
            })
        })
    }
}

impl<'a> QuestObjectiveAccessLikeCpp<'a> {
    /// Resolve the exact registry participants after a quest-completion visibility refresh.
    pub fn player_registry_sync_participants_like_cpp(
        &'a self,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_position: &'a Option<wow_core::Position>,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_health: &'a u32,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_max_health: &'a u32,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_alive: &'a bool,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_level: &'a u8,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixture_transport: &'a Option<Box<crate::session::PlayerTransportLoginStateLikeCpp>>,
    ) -> Option<(
        crate::session::PlayerRegistrySyncAccessLikeCpp<'a>,
        crate::session::PlayerRegistryControlBindingLikeCpp<'a>,
    )> {
        let guid = self.core.player_guid()?;
        let registry = self.core.player_registry()?;
        let position = self.core.player_registry_sync_access_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_position,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_health,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_max_health,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_alive,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_level,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_transport,
        );
        let control = self
            .core
            .player_registry_control_binding_like_cpp(guid, registry.as_ref());
        Some((position, control))
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn player_registry_hydration_access_like_cpp(
        &self,
    ) -> crate::session::PlayerRegistryHydrationAccessLikeCpp<'a> {
        self.core.player_registry_hydration_access_like_cpp()
    }
}
