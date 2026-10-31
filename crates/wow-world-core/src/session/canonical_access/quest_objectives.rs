// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Narrow canonical Player access for quest-objective progress operations.

use crate::session::state::SessionCore;
use wow_core::ObjectGuid;
use wow_entities::PlayerQuestStatusSaveTypeLikeCpp;

const QUEST_LOG_STATE_COMPLETE_LIKE_CPP: u32 = 0x0001;
const QUEST_LOG_OBJECTIVE_FLAG_BASE_LIKE_CPP: u32 = 256;

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
    /// Read the canonical Player-owned QuestLog rows for CREATE and visibility.
    pub fn quest_log_values_snapshot_like_cpp(
        &self,
    ) -> Option<[wow_entities::QuestLogValuesLikeCpp; 25]> {
        self.core
            .with_owned_player_like_cpp(|player| player.quest_log_values_like_cpp())
    }

    /// Read the live update-field QuestID, independently of quest-status slots.
    pub fn quest_slot_quest_id_like_cpp(&self, slot: u8) -> Option<u32> {
        self.core
            .with_owned_player_like_cpp(|player| player.quest_slot_quest_id_like_cpp(slot))
            .flatten()
    }

    pub fn set_quest_slot_like_cpp(&self, slot: u8, quest_id: u32) -> Option<bool> {
        self.core
            .with_owned_player_mut_like_cpp(|player| player.set_quest_slot_like_cpp(slot, quest_id))
    }

    pub fn set_quest_slot_counter_like_cpp(
        &self,
        slot: u8,
        counter: u8,
        count: u16,
    ) -> Option<bool> {
        self.core.with_owned_player_mut_like_cpp(|player| {
            player.set_quest_slot_counter_like_cpp(slot, counter, count)
        })
    }

    pub fn set_quest_slot_state_like_cpp(&self, slot: u8, state: u32) -> Option<bool> {
        self.core.with_owned_player_mut_like_cpp(|player| {
            player.set_quest_slot_state_like_cpp(slot, state)
        })
    }

    pub fn remove_quest_slot_state_like_cpp(&self, slot: u8, state: u32) -> Option<bool> {
        self.core.with_owned_player_mut_like_cpp(|player| {
            player.remove_quest_slot_state_like_cpp(slot, state)
        })
    }

    pub fn set_quest_slot_end_time_like_cpp(&self, slot: u8, end_time: i64) -> Option<bool> {
        self.core.with_owned_player_mut_like_cpp(|player| {
            player.set_quest_slot_end_time_like_cpp(slot, end_time)
        })
    }

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

    /// Resolve the credited player's race for a represented PvP objective kill.
    pub fn quest_credit_race_like_cpp(&self, credit_guid: ObjectGuid) -> Option<u8> {
        self.core.player_registry()?.quest_credit_race(credit_guid)
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
        self.core
            .player_is_game_master_with_fixture_like_cpp(fixture)
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

    /// Record `SetQuestObjectiveData`/`SetQuestStatus` dirty work through the strict Player owner.
    pub fn mark_quest_status_default_save_like_cpp(&self, quest_id: u32) -> Option<()> {
        self.core.with_owned_player_mut_like_cpp(|player| {
            if player
                .gameplay_state()
                .quests
                .status_like_cpp(quest_id)
                .is_some()
            {
                player
                    .gameplay_state_mut()
                    .quests
                    .mark_status_save_like_cpp(quest_id, PlayerQuestStatusSaveTypeLikeCpp::Default);
            }
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
        self.core.with_owned_player_mut_like_cpp(|player| {
            let (slot, count) = {
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
                (status.slot, status.objective_counts[objective_index])
            };
            if let Ok(counter) = u8::try_from(objective_index) {
                if slot < 25
                    && objective_index < 24
                    && player
                        .quest_slot_objective_data_for_save_like_cpp(
                            slot,
                            objective_index as i8,
                            false,
                        )
                        .is_some_and(|old| i32::from(old) != count)
                {
                    player
                        .gameplay_state_mut()
                        .quests
                        .mark_status_save_like_cpp(
                            quest_id,
                            PlayerQuestStatusSaveTypeLikeCpp::Default,
                        );
                }
                let _ = player.set_quest_slot_counter_like_cpp(slot, counter, count as u16);
            }
            Some(count)
        })
    }

    /// Set one canonical storing-flag objective and return its before/after state.
    pub fn update_storing_flag_like_cpp(
        &self,
        quest_id: u32,
        objective_index: usize,
        add_count: i32,
    ) -> Option<Option<(bool, bool)>> {
        self.core.with_owned_player_mut_like_cpp(|player| {
            let (slot, before, after) = {
                let status = player
                    .gameplay_state_mut()
                    .quests
                    .status_mut_like_cpp(quest_id)?;
                if status.objective_counts.len() <= objective_index {
                    status.objective_counts.resize(objective_index + 1, 0);
                }
                let before = status.objective_counts[objective_index] != 0;
                status.objective_counts[objective_index] = i32::from(add_count > 0);
                (
                    status.slot,
                    before,
                    status.objective_counts[objective_index] != 0,
                )
            };
            let state_flag = u32::try_from(objective_index)
                .ok()
                .and_then(|index| QUEST_LOG_OBJECTIVE_FLAG_BASE_LIKE_CPP.checked_shl(index))
                .unwrap_or(0);
            if slot < 25
                && objective_index < 24
                && player
                    .quest_slot_objective_data_for_save_like_cpp(slot, objective_index as i8, true)
                    .is_some_and(|old| i32::from(old) != i32::from(after))
            {
                player
                    .gameplay_state_mut()
                    .quests
                    .mark_status_save_like_cpp(quest_id, PlayerQuestStatusSaveTypeLikeCpp::Default);
            }
            if after {
                let _ = player.set_quest_slot_state_like_cpp(slot, state_flag);
            } else {
                let _ = player.remove_quest_slot_state_like_cpp(slot, state_flag);
            }
            Some((before, after))
        })
    }

    /// Revert one canonical completed quest when its threshold objective regresses.
    pub fn mark_quest_incomplete_if_complete_like_cpp(
        &self,
        quest_id: u32,
        complete_status: u8,
        incomplete_status: u8,
        mark_default: bool,
    ) -> Option<bool> {
        self.core.with_owned_player_mut_like_cpp(|player| {
            let should_change = player
                .gameplay_state()
                .quests
                .status_like_cpp(quest_id)
                .is_some_and(|status| status.status == complete_status);
            if !should_change {
                return false;
            }
            player.gameplay_state_mut().quests.set_status_like_cpp(
                quest_id,
                incomplete_status,
                mark_default,
            );
            // C++ IncompleteQuest changes status before consulting the
            // independent update-field QuestID lookup.
            if let Some(log_slot) = player.find_quest_slot_like_cpp(quest_id) {
                let _ = player
                    .remove_quest_slot_state_like_cpp(log_slot, QUEST_LOG_STATE_COMPLETE_LIKE_CPP);
            }
            true
        })
    }

    /// Complete one canonical quest status when it is still incomplete.
    pub fn complete_quest_status_like_cpp(
        &self,
        quest_id: u32,
        incomplete_status: u8,
        complete_status: u8,
        mark_default: bool,
    ) -> Option<Option<u8>> {
        self.core.with_owned_player_mut_like_cpp(|player| {
            let (old_status, slot) = {
                let status = player
                    .gameplay_state_mut()
                    .quests
                    .status_mut_like_cpp(quest_id)?;
                if status.status != incomplete_status {
                    return None;
                }
                (status.status, status.slot)
            };
            player.gameplay_state_mut().quests.set_status_like_cpp(
                quest_id,
                complete_status,
                mark_default,
            );
            let _ = player.set_quest_slot_state_like_cpp(slot, QUEST_LOG_STATE_COMPLETE_LIKE_CPP);
            Some(old_status)
        })
    }
}

impl<'a> QuestObjectiveAccessLikeCpp<'a> {
    /// Resolve the exact registry participants after a quest-completion visibility refresh.
    pub fn player_registry_sync_participants_like_cpp(
        &'a self,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_position: &'a Option<
            wow_core::Position,
        >,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_level: &'a u8,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_transport: &'a Option<
            Box<crate::session::PlayerTransportLoginStateLikeCpp>,
        >,
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
