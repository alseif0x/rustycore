//! Represented health and power changes and their regeneration.
//!
//! Moved out of the Session root under #617. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
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
}

#[cfg(test)]
#[path = "../../../unit_tests/session/combat/vitals/f3_shims.rs"]
mod f3_shims;
