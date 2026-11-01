// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! Instance packet definitions.

use crate::{ClientPacket, PacketError, ServerPacket, WorldPacket};
use wow_constants::{ClientOpcodes, ServerOpcodes};
use wow_core::ObjectGuid;

/// C++ `WorldPackets::Instance::InstanceLock`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceLockInfo {
    pub instance_id: u64,
    pub map_id: u32,
    pub difficulty_id: u32,
    pub time_remaining: i32,
    pub completed_mask: u32,
    pub locked: bool,
    pub extended: bool,
}

impl InstanceLockInfo {
    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.map_id);
        pkt.write_uint32(self.difficulty_id);
        pkt.write_uint64(self.instance_id);
        pkt.write_int32(self.time_remaining);
        pkt.write_uint32(self.completed_mask);
        pkt.write_bit(self.locked);
        pkt.write_bit(self.extended);
        pkt.flush_bits();
    }
}

/// C++ `WorldPackets::Instance::InstanceInfo` / `SMSG_INSTANCE_INFO`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InstanceInfo {
    pub locks: Vec<InstanceLockInfo>,
}

impl ServerPacket for InstanceInfo {
    const OPCODE: ServerOpcodes = ServerOpcodes::InstanceInfo;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.locks.len() as i32);
        for lock in &self.locks {
            lock.write(pkt);
        }
    }
}

/// C++ `WorldPackets::Instance::InstanceReset` / `SMSG_INSTANCE_RESET`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstanceReset {
    pub map_id: u32,
}

impl ServerPacket for InstanceReset {
    const OPCODE: ServerOpcodes = ServerOpcodes::InstanceReset;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.map_id);
    }
}

/// C++ `WorldPackets::Instance::InstanceResetFailed` / `SMSG_INSTANCE_RESET_FAILED`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstanceResetFailed {
    pub map_id: u32,
    pub reset_failed_reason: u8,
}

impl ServerPacket for InstanceResetFailed {
    const OPCODE: ServerOpcodes = ServerOpcodes::InstanceResetFailed;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.map_id);
        pkt.write_bits(u32::from(self.reset_failed_reason), 2);
        pkt.flush_bits();
    }
}

/// C++ `WorldPackets::Instance::ResetFailedNotify` / `SMSG_RESET_FAILED_NOTIFY`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ResetFailedNotify;

impl ServerPacket for ResetFailedNotify {
    const OPCODE: ServerOpcodes = ServerOpcodes::ResetFailedNotify;

    fn write(&self, _pkt: &mut WorldPacket) {}
}

/// C++ `RaidInstanceResetWarningType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RaidInstanceMessageType {
    WarningHours = 1,
    WarningMinutes = 2,
    WarningMinutesSoon = 3,
    Welcome = 4,
    Expired = 5,
}

/// C++ `WorldPackets::Instance::RaidInstanceMessage` / `SMSG_RAID_INSTANCE_MESSAGE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RaidInstanceMessage {
    pub message_type: RaidInstanceMessageType,
    pub map_id: u32,
    pub difficulty_id: u32,
    pub locked: bool,
    pub extended: bool,
}

impl ServerPacket for RaidInstanceMessage {
    const OPCODE: ServerOpcodes = ServerOpcodes::RaidInstanceMessage;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint8(self.message_type as u8);
        pkt.write_uint32(self.map_id);
        pkt.write_uint32(self.difficulty_id);
        pkt.write_bit(self.locked);
        pkt.write_bit(self.extended);
        pkt.flush_bits();
    }
}

/// C++ `WorldPackets::Instance::InstanceLockResponse` / `CMSG_INSTANCE_LOCK_RESPONSE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstanceLockResponse {
    pub accept_lock: bool,
}

impl ClientPacket for InstanceLockResponse {
    const OPCODE: ClientOpcodes = ClientOpcodes::InstanceLockResponse;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            accept_lock: pkt.read_bit()?,
        })
    }
}

/// C++ `WorldPackets::Instance::PendingRaidLock` / `SMSG_PENDING_RAID_LOCK`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingRaidLock {
    pub time_until_lock: i32,
    pub completed_mask: u32,
    pub extending: bool,
    pub warning_only: bool,
}

impl ServerPacket for PendingRaidLock {
    const OPCODE: ServerOpcodes = ServerOpcodes::PendingRaidLock;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.time_until_lock);
        pkt.write_uint32(self.completed_mask);
        pkt.write_bit(self.extending);
        pkt.write_bit(self.warning_only);
        pkt.flush_bits();
    }
}

/// C++ `WorldPackets::Instance::InstanceSaveCreated` / `SMSG_INSTANCE_SAVE_CREATED`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstanceSaveCreated {
    pub gm: bool,
}

impl ServerPacket for InstanceSaveCreated {
    const OPCODE: ServerOpcodes = ServerOpcodes::InstanceSaveCreated;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bit(self.gm);
        pkt.flush_bits();
    }
}

/// C++ `WorldPackets::Instance::InstanceEncounterEngageUnit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstanceEncounterEngageUnit {
    pub unit: ObjectGuid,
    pub target_frame_priority: u8,
}

impl ServerPacket for InstanceEncounterEngageUnit {
    const OPCODE: ServerOpcodes = ServerOpcodes::InstanceEncounterEngageUnit;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.unit);
        pkt.write_uint8(self.target_frame_priority);
    }
}

/// C++ `WorldPackets::Instance::InstanceEncounterDisengageUnit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstanceEncounterDisengageUnit {
    pub unit: ObjectGuid,
}

impl ServerPacket for InstanceEncounterDisengageUnit {
    const OPCODE: ServerOpcodes = ServerOpcodes::InstanceEncounterDisengageUnit;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.unit);
    }
}

/// C++ `WorldPackets::Instance::InstanceEncounterChangePriority`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstanceEncounterChangePriority {
    pub unit: ObjectGuid,
    pub target_frame_priority: u8,
}

impl ServerPacket for InstanceEncounterChangePriority {
    const OPCODE: ServerOpcodes = ServerOpcodes::InstanceEncounterChangePriority;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.unit);
        pkt.write_uint8(self.target_frame_priority);
    }
}

/// C++ `WorldPackets::Instance::InstanceEncounterStart`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstanceEncounterStart {
    pub in_combat_res_count: u32,
    pub max_in_combat_res_count: u32,
    pub combat_res_charge_recovery: u32,
    pub next_combat_res_charge_time: u32,
    pub in_progress: bool,
}

impl Default for InstanceEncounterStart {
    fn default() -> Self {
        Self {
            in_combat_res_count: 0,
            max_in_combat_res_count: 0,
            combat_res_charge_recovery: 0,
            next_combat_res_charge_time: 0,
            in_progress: true,
        }
    }
}

impl ServerPacket for InstanceEncounterStart {
    const OPCODE: ServerOpcodes = ServerOpcodes::InstanceEncounterStart;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.in_combat_res_count);
        pkt.write_uint32(self.max_in_combat_res_count);
        pkt.write_uint32(self.combat_res_charge_recovery);
        pkt.write_uint32(self.next_combat_res_charge_time);
        pkt.write_bit(self.in_progress);
        pkt.flush_bits();
    }
}

/// C++ `WorldPackets::Instance::InstanceEncounterEnd`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InstanceEncounterEnd;

impl ServerPacket for InstanceEncounterEnd {
    const OPCODE: ServerOpcodes = ServerOpcodes::InstanceEncounterEnd;

    fn write(&self, _pkt: &mut WorldPacket) {}
}

/// C++ `WorldPackets::Instance::InstanceEncounterInCombatResurrection`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InstanceEncounterInCombatResurrection;

impl ServerPacket for InstanceEncounterInCombatResurrection {
    const OPCODE: ServerOpcodes = ServerOpcodes::InstanceEncounterInCombatResurrection;

    fn write(&self, _pkt: &mut WorldPacket) {}
}

/// C++ `WorldPackets::Instance::InstanceEncounterGainCombatResurrectionCharge`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstanceEncounterGainCombatResurrectionCharge {
    pub in_combat_res_count: i32,
    pub combat_res_charge_recovery: u32,
}

impl ServerPacket for InstanceEncounterGainCombatResurrectionCharge {
    const OPCODE: ServerOpcodes = ServerOpcodes::InstanceEncounterGainCombatResurrectionCharge;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.in_combat_res_count);
        pkt.write_uint32(self.combat_res_charge_recovery);
    }
}

/// C++ `WorldPackets::Instance::BossKill` / `SMSG_BOSS_KILL`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BossKill {
    pub dungeon_encounter_id: u32,
}

impl ServerPacket for BossKill {
    const OPCODE: ServerOpcodes = ServerOpcodes::BossKill;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.dungeon_encounter_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_instance_info_matches_cpp_lock_count_only() {
        let mut pkt = WorldPacket::new_server(ServerOpcodes::InstanceInfo);

        InstanceInfo::default().write(&mut pkt);

        assert_eq!(&pkt.data()[2..], &[0, 0, 0, 0]);
    }

    #[test]
    fn instance_info_lock_serialization_matches_cpp_order_and_bits() {
        let mut pkt = WorldPacket::new_server(ServerOpcodes::InstanceInfo);

        InstanceInfo {
            locks: vec![InstanceLockInfo {
                instance_id: 0x0102_0304_0506_0708,
                map_id: 631,
                difficulty_id: 3,
                time_remaining: 3600,
                completed_mask: 0xA5,
                locked: true,
                extended: false,
            }],
        }
        .write(&mut pkt);

        assert_eq!(
            &pkt.data()[2..],
            &[
                1, 0, 0, 0, // LockList.size()
                0x77, 0x02, 0x00, 0x00, // MapID
                0x03, 0x00, 0x00, 0x00, // DifficultyID
                0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01, // InstanceID
                0x10, 0x0E, 0x00, 0x00, // TimeRemaining
                0xA5, 0x00, 0x00, 0x00, // CompletedMask
                0x80, // Locked=true, Extended=false
            ]
        );
    }

    #[test]
    fn instance_reset_serialization_matches_cpp() {
        let mut pkt = WorldPacket::new_server(ServerOpcodes::InstanceReset);

        InstanceReset { map_id: 631 }.write(&mut pkt);

        assert_eq!(&pkt.data()[2..], &[0x77, 0x02, 0x00, 0x00]);
    }

    #[test]
    fn instance_reset_failed_serialization_matches_cpp_map_then_two_reason_bits() {
        let mut pkt = WorldPacket::new_server(ServerOpcodes::InstanceResetFailed);

        InstanceResetFailed {
            map_id: 631,
            reset_failed_reason: 1,
        }
        .write(&mut pkt);

        assert_eq!(&pkt.data()[2..], &[0x77, 0x02, 0x00, 0x00, 0x40]);
    }

    #[test]
    fn reset_failed_notify_serialization_matches_cpp_opcode_and_empty_body() {
        let bytes = ResetFailedNotify.to_bytes();

        assert_eq!(ResetFailedNotify::OPCODE, ServerOpcodes::ResetFailedNotify);
        assert_eq!(
            bytes,
            (ServerOpcodes::ResetFailedNotify as u16).to_le_bytes()
        );
    }

    #[test]
    fn raid_instance_message_serialization_matches_cpp_order_and_bits() {
        let mut pkt = WorldPacket::new_server(ServerOpcodes::RaidInstanceMessage);

        RaidInstanceMessage {
            message_type: RaidInstanceMessageType::Welcome,
            map_id: 631,
            difficulty_id: 3,
            locked: true,
            extended: true,
        }
        .write(&mut pkt);

        assert_eq!(
            &pkt.data()[2..],
            &[
                0x04, // Type = RAID_INSTANCE_WELCOME
                0x77, 0x02, 0x00, 0x00, // MapID
                0x03, 0x00, 0x00, 0x00, // DifficultyID
                0xC0, // Locked=true, Extended=true
            ]
        );
    }

    #[test]
    fn instance_lock_response_reads_cpp_accept_bit() {
        let mut pkt = WorldPacket::from_bytes(&[0x80]);

        let response = InstanceLockResponse::read(&mut pkt).unwrap();

        assert!(response.accept_lock);
    }

    #[test]
    fn pending_raid_lock_serialization_matches_cpp() {
        let mut pkt = WorldPacket::new_server(ServerOpcodes::PendingRaidLock);

        PendingRaidLock {
            time_until_lock: 60_000,
            completed_mask: 0xA5,
            extending: true,
            warning_only: false,
        }
        .write(&mut pkt);

        assert_eq!(
            &pkt.data()[2..],
            &[
                0x60, 0xEA, 0x00, 0x00, // TimeUntilLock
                0xA5, 0x00, 0x00, 0x00, // CompletedMask
                0x80, // Extending=true, WarningOnly=false
            ]
        );
    }

    #[test]
    fn instance_save_created_serialization_matches_cpp_gm_bit() {
        let mut pkt = WorldPacket::new_server(ServerOpcodes::InstanceSaveCreated);

        InstanceSaveCreated { gm: true }.write(&mut pkt);

        assert_eq!(&pkt.data()[2..], &[0x80]);
    }

    #[test]
    fn instance_encounter_unit_packets_match_cpp_packed_guid_order() {
        let mut engage = WorldPacket::new_server(ServerOpcodes::InstanceEncounterEngageUnit);
        InstanceEncounterEngageUnit {
            unit: ObjectGuid::EMPTY,
            target_frame_priority: 7,
        }
        .write(&mut engage);
        assert_eq!(&engage.data()[2..], &[0x00, 0x00, 0x07]);

        let mut disengage = WorldPacket::new_server(ServerOpcodes::InstanceEncounterDisengageUnit);
        InstanceEncounterDisengageUnit {
            unit: ObjectGuid::EMPTY,
        }
        .write(&mut disengage);
        assert_eq!(&disengage.data()[2..], &[0x00, 0x00]);

        let mut priority = WorldPacket::new_server(ServerOpcodes::InstanceEncounterChangePriority);
        InstanceEncounterChangePriority {
            unit: ObjectGuid::EMPTY,
            target_frame_priority: 2,
        }
        .write(&mut priority);
        assert_eq!(&priority.data()[2..], &[0x00, 0x00, 0x02]);
    }

    #[test]
    fn instance_encounter_start_and_end_packets_match_cpp() {
        let mut start = WorldPacket::new_server(ServerOpcodes::InstanceEncounterStart);
        InstanceEncounterStart {
            in_combat_res_count: 1,
            max_in_combat_res_count: 9,
            combat_res_charge_recovery: 30_000,
            next_combat_res_charge_time: 60_000,
            in_progress: true,
        }
        .write(&mut start);
        assert_eq!(
            &start.data()[2..],
            &[
                0x01, 0x00, 0x00, 0x00, // InCombatResCount
                0x09, 0x00, 0x00, 0x00, // MaxInCombatResCount
                0x30, 0x75, 0x00, 0x00, // CombatResChargeRecovery
                0x60, 0xEA, 0x00, 0x00, // NextCombatResChargeTime
                0x80, // InProgress=true
            ]
        );

        let mut end = WorldPacket::new_server(ServerOpcodes::InstanceEncounterEnd);
        InstanceEncounterEnd.write(&mut end);
        assert!(end.data()[2..].is_empty());
    }

    #[test]
    fn combat_resurrection_and_boss_kill_packets_match_cpp() {
        let mut in_combat =
            WorldPacket::new_server(ServerOpcodes::InstanceEncounterInCombatResurrection);
        InstanceEncounterInCombatResurrection.write(&mut in_combat);
        assert!(in_combat.data()[2..].is_empty());

        let mut gain =
            WorldPacket::new_server(ServerOpcodes::InstanceEncounterGainCombatResurrectionCharge);
        InstanceEncounterGainCombatResurrectionCharge {
            in_combat_res_count: -1,
            combat_res_charge_recovery: 45_000,
        }
        .write(&mut gain);
        assert_eq!(
            &gain.data()[2..],
            &[
                0xFF, 0xFF, 0xFF, 0xFF, // InCombatResCount
                0xC8, 0xAF, 0x00, 0x00, // CombatResChargeRecovery
            ]
        );

        let mut boss = WorldPacket::new_server(ServerOpcodes::BossKill);
        BossKill {
            dungeon_encounter_id: 1234,
        }
        .write(&mut boss);
        assert_eq!(&boss.data()[2..], &[0xD2, 0x04, 0x00, 0x00]);
    }
}
