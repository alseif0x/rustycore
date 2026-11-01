// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::SessionLifecycleState;
use wow_entities::Player;
use wow_world_core::session::HubRef;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_world_core::session::{
    power_type_from_u8_like_cpp, primary_power_type_for_player_class_like_cpp,
};

impl SessionLifecycleState {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn apply_represented_player_powers_to_canonical_like_cpp(
        &self,
        hub: HubRef<'_>,
        player: &mut Player,
    ) {
        let powers = hub
            .fixtures
            .combat
            .represented_player_powers_like_cpp
            .map(|value| value.unwrap_or(0));
        let max_powers = hub
            .fixtures
            .combat
            .represented_player_max_powers_like_cpp
            .map(|value| value.unwrap_or(0));
        player
            .unit_mut()
            .replace_create_power_arrays_like_cpp(powers, max_powers);
        let Some(current) = hub.fixtures.combat.represented_player_powers_like_cpp[0] else {
            return;
        };
        let primary_power_type =
            primary_power_type_for_player_class_like_cpp(hub.player_class_like_cpp());
        for raw_power in 0..=25 {
            player.set_power_index(power_type_from_u8_like_cpp(raw_power), None);
        }
        player.set_power_index(primary_power_type, Some(0));
        player.unit_mut().set_display_power(primary_power_type);
        player.unit_mut().set_create_mana_like_cpp(
            hub.fixtures
                .combat
                .represented_player_base_mana_like_cpp
                .max(0),
        );
        if let Some(max) = hub.fixtures.combat.represented_player_max_powers_like_cpp[0] {
            player.unit_mut().set_max_power(primary_power_type, max);
            player.unit_mut().set_power(primary_power_type, current);
        }
    }

    pub fn apply_represented_player_unit_shape_to_canonical_like_cpp(
        &self,
        hub: HubRef<'_>,
        player: &mut Player,
    ) {
        let display_id = wow_world_core::session::default_display_id(
            hub.player_race_like_cpp(),
            hub.player_gender_like_cpp(),
        );
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let mount_display_id = 0;
        #[cfg(any(test, feature = "test-fixtures"))]
        let mount_display_id =
            u32::try_from(hub.fixtures.vehicles.player_mount_display_id_like_cpp).unwrap_or(0);
        let unit = player.unit_mut();
        unit.set_display_id(display_id, true);
        unit.set_mount_display_id(mount_display_id);
        #[cfg(not(any(test, feature = "test-fixtures")))]
        unit.set_collision_height_like_cpp(1.0);
        #[cfg(any(test, feature = "test-fixtures"))]
        unit.set_collision_height_like_cpp(hub.fixtures.movement.player_collision_height_like_cpp);
        #[cfg(not(any(test, feature = "test-fixtures")))]
        unit.world_mut().object_mut().set_scale(1.0);
        #[cfg(any(test, feature = "test-fixtures"))]
        unit.world_mut()
            .object_mut()
            .set_scale(hub.fixtures.presentation.player_object_scale_like_cpp);
    }
}
