// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use crate::session::state::SessionCore;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_core::ObjectGuid;

/// Narrow access for the canonical Player inputs used by directory hydration.
///
/// The Player and SessionCore remain private; callers receive only the strict quest snapshot,
/// fixture-owner proof, and the existing GUID-resolved hydration operation.
#[derive(Clone, Copy)]
pub struct PlayerRegistryHydrationAccessLikeCpp<'a> {
    core: &'a SessionCore,
}

impl SessionCore {
    pub fn player_registry_hydration_access_like_cpp(
        &self,
    ) -> PlayerRegistryHydrationAccessLikeCpp<'_> {
        PlayerRegistryHydrationAccessLikeCpp { core: self }
    }
}

impl PlayerRegistryHydrationAccessLikeCpp<'_> {
    /// The strict canonical quest snapshot. A stale or missing handle remains unresolved.
    pub fn owned_player_quest_gameplay_snapshot_like_cpp(
        &self,
    ) -> Option<wow_entities::PlayerQuestGameplayState> {
        self.core
            .with_owned_player_like_cpp(|player| player.gameplay_state().quests.clone())
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn owner_handle_absent_like_cpp(&self) -> bool {
        self.core.player_handle_like_cpp.is_none()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn hydrate_player_directory_fixture_like_cpp(
        &self,
        known_spells: Vec<i32>,
        quests: Option<wow_entities::PlayerQuestGameplayState>,
        mount_vehicle_kit: Option<wow_entities::Vehicle>,
        vehicle_seat_flags: Option<i32>,
        vehicle_seat_id: Option<u32>,
        pet_guid: Option<ObjectGuid>,
    ) {
        let _ = self.core.mutate_canonical_player_like_cpp(|player| {
            let state = player.gameplay_state_mut();
            let rows = known_spells
                .iter()
                .copied()
                .map(|spell_id| {
                    (
                        spell_id,
                        wow_entities::PlayerKnownSpellRecord {
                            spell_id,
                            state: wow_entities::PlayerSpellLoadState::Unchanged,
                            active: true,
                            disabled: false,
                            favorite: false,
                            dependent: false,
                        },
                    )
                })
                .collect();
            state
                .spells
                .replace_known_spells_and_rows_like_cpp(known_spells.clone(), rows);
            if let Some(quests) = quests.clone() {
                state.quests = quests;
            }
            state.mount_vehicle_kit = mount_vehicle_kit.clone();
            state.vehicle_seat_flags = vehicle_seat_flags;
            state.vehicle_seat_id = vehicle_seat_id;
            state.pet_guid = pet_guid;
        });
    }
}
