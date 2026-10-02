//! Represented health and power changes and their regeneration.
//!
//! Moved out of the Session root under #617. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn canonical_player_health_snapshot_like_cpp(&self) -> Option<(u32, u32)> {
        self.core.canonical_player_snapshot_like_cpp(|player| {
            (
                player.unit().data().health.min(u64::from(u32::MAX)) as u32,
                player.unit().data().max_health.min(u64::from(u32::MAX)) as u32,
            )
        })
    }
    /// C++ `Unit::SetPower`'s `SMSG_POWER_UPDATE` publication
    /// (`Unit.cpp:9287-9312`). C++ also marks the changed `UnitData::Power`
    /// field, which the canonical Player setter already did, and sends the same
    /// packet to the nearby observers through `SendMessageToSet(packet, true)`;
    /// this owner session sends its own copy and fans the identical bytes out
    /// through the realm visibility rail.
    pub(in crate::session) fn send_player_power_update_like_cpp(
        &self,
        guid: ObjectGuid,
        power: PowerType,
        value: i32,
    ) {
        use wow_packet::ServerPacket;
        let packet = wow_packet::packets::combat::PowerUpdate {
            guid,
            powers: vec![(value, power as u8)],
        };
        self.send_packet(&packet);
        self.broadcast_player_packet_to_visible_set_realm_like_cpp(packet.to_bytes());
    }
    /// Installs the process-owned DB2 `PowerType` catalog. C++
    /// `sDB2Manager.GetPowerTypeEntry(power)` (`Unit.cpp:6581-6585`); the
    /// spawn/regeneration paths read the same store through
    /// `CreatureSpawnCatalogsLikeCpp`, so this slot is the session's read-only
    /// handle for the spell-effect chain.
    pub fn set_power_type_store(&mut self, store: Arc<PowerTypeStore>) {
        self.catalogs.power_type_store = Some(store);
    }
    pub(crate) fn represented_player_power_values_like_cpp(
        &self,
    ) -> Option<[i32; MAX_POWERS_PER_CLASS]> {
        let canonical = self.resolved_player_power_values_like_cpp();
        #[cfg(test)]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return character_power_snapshot_values_like_cpp(
                &self.fixtures.combat.represented_player_powers_like_cpp,
            );
        }
        canonical
    }
    fn resolved_player_power_values_like_cpp(&self) -> Option<[i32; MAX_POWERS_PER_CLASS]> {
        let canonical = self
            .core
            .with_owned_player_like_cpp(|player| player.unit().data().power);
        #[cfg(test)]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return self
                .core
                .mutate_canonical_player_like_cpp(|player| player.unit().data().power);
        }
        canonical
    }
    pub(in crate::session) fn resolved_player_power_snapshot_like_cpp(
        &self,
    ) -> Option<CharacterPowerSnapshotLikeCpp> {
        let canonical = self
            .resolved_player_power_values_like_cpp()
            .map(loaded_character_power_snapshot_like_cpp);
        #[cfg(test)]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(self.fixtures.combat.represented_player_powers_like_cpp);
        }
        canonical
    }
    pub(crate) fn set_player_health_like_cpp(&mut self, health: u32, max_health: u32) {
        let _ =
            crate::session::hub_mut(self).sync_canonical_player_health_like_cpp(health, max_health);
        self.sync_player_registry_state_like_cpp();
    }
    #[cfg(test)]
    pub(crate) fn player_max_health_like_cpp(&self) -> u32 {
        crate::session::hub_ref(self)
            .resolved_player_vitals_like_cpp()
            .unwrap()
            .1
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/combat/vitals/f3_shims.rs"]
mod f3_shims;
