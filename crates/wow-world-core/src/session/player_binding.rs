// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player binding data shared with the WorldSession shell.

use wow_core::ObjectGuid;

#[cfg(any(test, feature = "test-fixtures"))]
pub struct PlayerTransportLoginStateLikeCpp {
    pub info: wow_packet::packets::movement::TransportInfo,
}

/// Login-only identity input consumed while the canonical Player is being
/// constructed. Once a generation-checked Player exists this value is retired;
/// it is not a second runtime identity authority.
#[derive(Debug, Clone, Default)]
pub struct PlayerIdentityBootstrapLikeCpp {
    pub name: Option<String>,
    pub race: u8,
    pub class: u8,
    pub level: u8,
    pub gender: u8,
}

impl crate::session::state::SessionCore {
    pub fn set_canonical_chosen_title_like_cpp(
        &mut self,
        title_id: i32,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        self.mutate_canonical_player_like_cpp(|player| {
            player.set_chosen_title_like_cpp(title_id);
            player.values_update(true)
        })
    }

    pub fn player_is_possessing_like_cpp(&self) -> bool {
        let Some(player_guid) = self.player_guid else {
            return false;
        };
        let map_id = u32::from(self.player_map_id_like_cpp());
        let Some(manager) = self.canonical_map_manager.as_ref() else {
            return false;
        };
        let Ok(manager) = manager.lock() else {
            return false;
        };

        let mut result = None;
        manager.do_for_all_maps_with_map_id(map_id, |managed| {
            if result.is_some() {
                return;
            }

            let map = managed.map();
            let Some(player) = map.get_typed_player(player_guid) else {
                return;
            };
            let Some(charmed_guid) = player.unit().subsystems().control.charmed_guid else {
                result = Some(false);
                return;
            };

            let target_possessed_by_player = map
                .get_typed_player(charmed_guid)
                .map(|target| {
                    let control = &target.unit().subsystems().control;
                    control.charmer_guid == Some(player_guid) && control.is_possessed()
                })
                .or_else(|| {
                    map.with_creature_like_cpp(charmed_guid, |target| {
                        let control = &target.unit().subsystems().control;
                        control.charmer_guid == Some(player_guid) && control.is_possessed()
                    })
                })
                .unwrap_or(false);

            result = Some(target_possessed_by_player);
        });

        result.unwrap_or(false)
    }

    pub fn represented_player_charmed_guid_like_cpp(&self) -> ObjectGuid {
        let Some(player_guid) = self.player_guid else {
            return ObjectGuid::EMPTY;
        };
        let map_id = u32::from(self.player_map_id_like_cpp());
        let Some(manager) = self.canonical_map_manager.as_ref() else {
            return ObjectGuid::EMPTY;
        };
        let Ok(manager) = manager.lock() else {
            return ObjectGuid::EMPTY;
        };

        let mut result = None;
        manager.do_for_all_maps_with_map_id(map_id, |managed| {
            if result.is_some() {
                return;
            }

            let Some(player) = managed.map().get_typed_player(player_guid) else {
                return;
            };
            result = Some(
                player
                    .unit()
                    .subsystems()
                    .control
                    .charmed_guid
                    .unwrap_or(ObjectGuid::EMPTY),
            );
        });

        result.unwrap_or(ObjectGuid::EMPTY)
    }

    pub fn player_liquid_status_like_cpp(&self) -> Option<u32> {
        self.canonical_player_snapshot_like_cpp(|player| player.gameplay_state().liquid_status)
    }

    /// Get the logged-in player GUID.
    pub fn player_guid(&self) -> Option<ObjectGuid> {
        self.player_guid
    }
}
