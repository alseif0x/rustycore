use crate::entity_update_bridge::player_values_update_to_update_object;
use crate::session::power_type_from_u8_like_cpp;
use crate::session::state::SessionCore;
use wow_constants::PowerType;
use wow_core::ObjectGuid;
use wow_data::character_progression::PowerTypeStore;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_entities::MAX_POWERS_PER_CLASS;
use wow_entities::{Player, UNIT_DATA_HEALTH_BIT, UnitDataUpdate, UnitDataValues, UpdateMask};

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

    pub fn send_player_health_values_update_like_cpp(&self, guid: ObjectGuid, health: u64) {
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

impl crate::session::HubRef<'_> {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn player_health_like_cpp(&self) -> u32 {
        self.resolved_player_vitals_like_cpp().unwrap().0
    }
}

impl crate::session::state::SessionCatalogs {
    pub fn power_type_store_like_cpp(&self) -> Option<&PowerTypeStore> {
        self.power_type_store.as_deref()
    }
}

impl crate::session::HubMut<'_> {
    pub fn sync_canonical_player_primary_power_like_cpp(
        &mut self,
        power_type: PowerType,
        current: i32,
        max: i32,
        base_mana: i32,
    ) -> bool {
        let synced = self
            .core
            .with_owned_player_mut_for_power_like_cpp(|player| {
                for raw_power in 0..=25 {
                    player.set_power_index(power_type_from_u8_like_cpp(raw_power), None);
                }
                player.set_power_index(power_type, Some(0));
                player.unit_mut().set_display_power(power_type);
                player.unit_mut().set_create_mana_like_cpp(base_mana.max(0));
                player.unit_mut().set_max_power(power_type, max.max(0));
                player.unit_mut().set_power(power_type, current.max(0));
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if synced || self.core.player_handle_like_cpp.is_none() {
            self.fixtures.combat.represented_player_base_mana_like_cpp = base_mana.max(0);
            self.set_represented_player_power_slot_like_cpp(0, current, Some(max));
        }
        synced
    }
}

impl crate::session::HubMut<'_> {
    pub fn sync_canonical_player_health_like_cpp(
        &mut self,
        health: u32,
        max_health: u32,
    ) -> Option<(u32, u32)> {
        let max_health = max_health.max(1);
        let health = health.min(max_health);
        let canonical = self.core.with_owned_player_mut_like_cpp(|player| {
            if health == 0 {
                player
                    .unit_mut()
                    .set_death_state(wow_constants::DeathState::Corpse);
            } else if matches!(
                player.unit().death_state(),
                wow_constants::DeathState::JustDied | wow_constants::DeathState::Corpse
            ) {
                player
                    .unit_mut()
                    .set_death_state(wow_constants::DeathState::Alive);
            }
            player.unit_mut().set_max_health(u64::from(max_health));
            player.unit_mut().set_health(u64::from(health));
            (
                player.unit().data().health.min(u64::from(u32::MAX)) as u32,
                player.unit().data().max_health.min(u64::from(u32::MAX)) as u32,
            )
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        let result = canonical.or_else(|| {
            if self.core.player_handle_like_cpp.is_some() {
                return None;
            }
            self.core.mutate_canonical_player_like_cpp(|player| {
                player.unit_mut().set_death_state(if health == 0 {
                    wow_constants::DeathState::Corpse
                } else {
                    wow_constants::DeathState::Alive
                });
                player.unit_mut().set_max_health(u64::from(max_health));
                player.unit_mut().set_health(u64::from(health));
                (
                    player.unit().data().health.min(u64::from(u32::MAX)) as u32,
                    player.unit().data().max_health.min(u64::from(u32::MAX)) as u32,
                )
            })
        });
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let result = canonical;
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            let (current, max) = result.unwrap_or((health, max_health));
            self.fixtures.combat.player_health_like_cpp = current;
            self.fixtures.combat.player_max_health_like_cpp = max;
            self.fixtures.combat.player_alive_like_cpp = current > 0;
        }
        result
    }
}

impl crate::session::HubMut<'_> {
    pub fn sync_canonical_player_primary_power_max_like_cpp(
        &mut self,
        power_type: PowerType,
        max: i32,
        base_mana: i32,
    ) -> Option<(i32, i32)> {
        let result = self
            .core
            .with_owned_player_mut_for_power_like_cpp(|player| {
                if player.unit().get_power_index(power_type).is_none() {
                    player.set_power_index(power_type, Some(0));
                }
                player.unit_mut().set_display_power(power_type);
                player.unit_mut().set_create_mana_like_cpp(base_mana.max(0));
                // C++ `Unit::SetMaxPower` updates max and clamps current if needed.
                player.unit_mut().set_max_power(power_type, max.max(0));
                (
                    player.unit().get_power(power_type),
                    player.unit().get_max_power(power_type),
                )
            });
        #[cfg(any(test, feature = "test-fixtures"))]
        if let Some((current, max)) = result.or_else(|| {
            (self.core.player_handle_like_cpp.is_none()).then_some((
                self.fixtures.combat.represented_player_powers_like_cpp[0].unwrap_or(0),
                max.max(0),
            ))
        }) {
            self.fixtures.combat.represented_player_base_mana_like_cpp = base_mana.max(0);
            self.set_represented_player_power_slot_like_cpp(0, current, Some(max));
        }
        result
    }

    pub fn sync_canonical_player_max_health_like_cpp(
        &mut self,
        max_health: u32,
    ) -> Option<(u32, u32)> {
        let max_health = max_health.max(1);
        let canonical = self.core.with_owned_player_mut_like_cpp(|player| {
            // C++ `Unit::SetMaxHealth` updates max and clamps current only if needed.
            player.unit_mut().set_max_health(u64::from(max_health));
            (
                player.unit().data().health.min(u64::from(u32::MAX)) as u32,
                player.unit().data().max_health.min(u64::from(u32::MAX)) as u32,
            )
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        let result = canonical.or_else(|| {
            if self.core.player_handle_like_cpp.is_some() {
                return None;
            }
            self.core
                .mutate_canonical_player_like_cpp(|player| {
                    player.unit_mut().set_max_health(u64::from(max_health));
                    (
                        player.unit().data().health.min(u64::from(u32::MAX)) as u32,
                        player.unit().data().max_health.min(u64::from(u32::MAX)) as u32,
                    )
                })
                .or_else(|| {
                    Some((
                        self.fixtures.combat.player_health_like_cpp.min(max_health),
                        max_health,
                    ))
                })
        });
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let result = canonical;
        #[cfg(any(test, feature = "test-fixtures"))]
        if let Some((current, max)) = result {
            self.fixtures.combat.player_health_like_cpp = current;
            self.fixtures.combat.player_max_health_like_cpp = max;
            self.fixtures.combat.player_alive_like_cpp = current > 0;
        }
        result
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_represented_player_power_slot_like_cpp(
        &mut self,
        slot: usize,
        current: i32,
        max: Option<i32>,
    ) {
        if slot >= MAX_POWERS_PER_CLASS {
            return;
        }
        self.fixtures.combat.represented_player_powers_like_cpp[slot] = Some(current.max(0));
        if let Some(max) = max {
            self.fixtures.combat.represented_player_max_powers_like_cpp[slot] = Some(max.max(0));
        }
    }
}
