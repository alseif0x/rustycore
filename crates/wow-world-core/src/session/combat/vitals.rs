use crate::entity_update_bridge::player_values_update_to_update_object;
use crate::session::state::SessionCore;
use wow_constants::PowerType;
use wow_core::ObjectGuid;
use wow_entities::{
    Player, UNIT_DATA_HEALTH_BIT, UnitDataUpdate, UnitDataValues, UpdateMask,
};

impl SessionCore {
    /// Test fixtures created before #578 may inject a typed Player directly
    /// into a synthetic MapManager without installing its owner handle. Keep
    /// that compatibility outside production; an existing stale handle never
    /// falls back to GUID lookup.
    pub fn with_owned_player_mut_for_power_like_cpp<R>(
        &self,
        f: impl FnOnce(&mut Player) -> R,
    ) -> Option<R> {
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.player_handle_like_cpp.is_none() {
            return self.mutate_canonical_player_like_cpp(f);
        }
        self.with_owned_player_mut_like_cpp(f)
    }

    pub fn canonical_player_power_snapshot_like_cpp(
        &self,
        power_type: PowerType,
    ) -> Option<(i32, i32)> {
        self.canonical_player_snapshot_like_cpp(|player| {
            (
                player.unit().get_power(power_type),
                player.unit().get_max_power(power_type),
            )
        })
    }

    pub fn send_player_health_values_update_like_cpp(
        &self,
        guid: ObjectGuid,
        health: u64,
    ) {
        let mut mask = UpdateMask::new(UNIT_DATA_HEALTH_BIT + 1);
        mask.set(UNIT_DATA_HEALTH_BIT);
        let update = wow_entities::PlayerValuesUpdate {
            changed_object_type_mask: 0,
            object_data: None,
            unit_data: Some(UnitDataUpdate {
                mask,
                values: UnitDataValues {
                    health,
                    ..Default::default()
                },
            }),
            player_data: None,
            active_player_data: None,
        };

        if let Some(packet) =
            player_values_update_to_update_object(guid, self.player_map_id_like_cpp(), &update)
        {
            self.send_packet(&packet);
        }
    }

    pub fn send_player_health_update_like_cpp(&self, guid: ObjectGuid, health: u64) {
        self.send_packet(&wow_packet::packets::combat::HealthUpdate {
            guid,
            health: health.min(i64::MAX as u64) as i64,
        });
    }
}
