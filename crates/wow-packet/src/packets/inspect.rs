// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Inspect packet definitions: inspect request/response packets.

use crate::{ClientPacket, PacketError, ServerPacket, WorldPacket};
use wow_constants::{ClientOpcodes, ServerOpcodes};
use wow_core::ObjectGuid;

/// CMSG_REQUEST_HONOR_STATS.
///
/// C++ `WorldPackets::Inspect::RequestHonorStats::Read` reads a target
/// `ObjectGuid`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestHonorStats {
    pub target: ObjectGuid,
}

impl ClientPacket for RequestHonorStats {
    const OPCODE: ClientOpcodes = ClientOpcodes::RequestHonorStats;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            target: packet.read_packed_guid()?,
        })
    }
}

/// An equipped item for the inspect result.
pub struct InspectItem {
    pub slot: u8,
    pub item_id: i32,
}

/// SMSG_INSPECT_RESULT (0x2631)
pub struct InspectResult {
    pub target_guid: ObjectGuid,
    pub target_name: String,
    pub race: u8,
    pub class_id: u8,
    pub gender: u8,
    pub level: u32,
    pub items: Vec<InspectItem>,
}

impl ServerPacket for InspectResult {
    const OPCODE: ServerOpcodes = ServerOpcodes::InspectResult;

    fn write(&self, w: &mut WorldPacket) {
        // ── PlayerModelDisplayInfo.Write() ──
        w.write_packed_guid(&self.target_guid);
        w.write_int32(0); // SpecializationID
        w.write_int32(self.items.len() as i32); // Items.Count
        let name_bytes = self.target_name.as_bytes();
        w.write_bits(name_bytes.len() as u32, 6);
        w.write_uint8(self.gender);
        w.write_uint8(self.race);
        w.write_uint8(self.class_id);
        w.write_int32(0); // Customizations.Count = 0
        // No FlushBits here — Name string follows immediately after the bits
        w.write_bytes(name_bytes);
        // (no customizations — loop is empty)

        // ── InspectItemData.Write() for each equipped item ──
        for item in &self.items {
            w.write_packed_guid(&ObjectGuid::EMPTY); // CreatorGUID
            w.write_uint8(item.slot); // Index
            w.write_int32(0); // AzeritePowers.Count
            w.write_int32(0); // AzeriteEssences.Count
            // ItemInstance.Write():
            w.write_int32(item.item_id); // ItemID
            w.write_int32(0); // RandomPropertiesSeed
            w.write_int32(0); // RandomPropertiesID
            w.write_bit(false); // ItemBonus != null → false
            w.flush_bits();
            // ItemModList.Write() — empty:
            w.write_bits(0u32, 6); // Values.Count = 0
            w.flush_bits();
            // Back to InspectItemData:
            w.write_bit(true); // Usable = true
            w.write_bits(0u32, 4); // Enchants.Count = 0
            w.write_bits(0u32, 2); // Gems.Count = 0
            w.flush_bits();
            // (no AzeriteEssences, no Enchants, no Gems)
        }

        // ── InspectResult.Write() continues ──
        w.write_int32(0); // Glyphs.Count
        w.write_int32(0); // Talents.Count
        w.write_int32(0); // ItemLevel
        w.write_uint8(0); // LifetimeMaxRank
        w.write_uint16(0); // TodayHK
        w.write_uint16(0); // YesterdayHK
        w.write_int32(0); // LifetimeHK
        w.write_int32(0); // HonorLevel
        // (no glyphs, no talents — loops empty)
        w.write_bit(false); // GuildData.HasValue = false
        w.write_bit(false); // AzeriteLevel.HasValue = false
        w.flush_bits();

        // 7x PVPBracketData (all empty, bracket index 0..7)
        for bracket in 0u8..7 {
            w.write_uint8(bracket); // Bracket
            w.write_int32(0); // Unused3
            w.write_int32(0); // Rating
            w.write_int32(0); // Rank
            w.write_int32(0); // WeeklyPlayed
            w.write_int32(0); // WeeklyWon
            w.write_int32(0); // SeasonPlayed
            w.write_int32(0); // SeasonWon
            w.write_int32(0); // WeeklyBestRating
            w.write_int32(0); // SeasonBestRating
            w.write_int32(0); // PvpTierID
            w.write_int32(0); // WeeklyBestWinPvpTierID
            w.write_int32(0); // Unused1
            w.write_int32(0); // Unused2
            w.write_int32(0); // RoundsSeasonPlayed
            w.write_int32(0); // RoundsSeasonWon
            w.write_int32(0); // RoundsWeeklyPlayed
            w.write_int32(0); // RoundsWeeklyWon
            w.write_bit(false); // Disqualified
            w.flush_bits();
        }

        // TraitInspectInfo.Write():
        w.write_int32(self.level as i32); // Level
        w.write_int32(0); // ChrSpecializationID
        // TraitConfigPacket.Write() — empty (ID=0, Type=0):
        w.write_int32(0); // ID
        w.write_int32(0); // Type = 0 (None — no switch case fires)
        w.write_int32(0); // Entries.Count
        // Name bits:
        w.write_bits(0u32, 9); // Name length = 0
        w.flush_bits();
        // WriteString("") — empty, nothing written
    }
}

/// SMSG_INSPECT_HONOR_STATS.
///
/// C++ `WorldPackets::Inspect::InspectHonorStatsResponse::Write` serializes:
/// `Target`, `LifetimeHK`, `TodayContribution`, `YesterdayContribution`,
/// `TodayHK`, `YesterdayHK`, `LifetimeMaxRank`, `HonorLevel`.
pub struct InspectHonorStatsResponse {
    pub target: ObjectGuid,
    pub lifetime_hk: u32,
    pub today_contribution: u32,
    pub yesterday_contribution: u32,
    pub today_hk: u16,
    pub yesterday_hk: u16,
    pub lifetime_max_rank: u32,
    pub honor_level: u32,
}

impl ServerPacket for InspectHonorStatsResponse {
    const OPCODE: ServerOpcodes = ServerOpcodes::InspectHonorStats;

    fn write(&self, w: &mut WorldPacket) {
        w.write_packed_guid(&self.target);
        w.write_uint32(self.lifetime_hk);
        w.write_uint32(self.today_contribution);
        w.write_uint32(self.yesterday_contribution);
        w.write_uint16(self.today_hk);
        w.write_uint16(self.yesterday_hk);
        w.write_uint32(self.lifetime_max_rank);
        w.write_uint32(self.honor_level);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_constants::ServerOpcodes;

    #[test]
    fn request_honor_stats_reads_target_guid_like_cpp() {
        let guid = ObjectGuid::create_player(1, 0x1234);
        let mut writer = WorldPacket::new_empty();
        writer.write_packed_guid(&guid);

        let mut packet = WorldPacket::from_bytes(&writer.into_data());
        let parsed = RequestHonorStats::read(&mut packet).expect("request honor stats");

        assert_eq!(parsed.target, guid);
    }

    #[test]
    fn inspect_honor_stats_response_writes_cpp_order() {
        let guid = ObjectGuid::create_player(1, 0x55);
        let bytes = InspectHonorStatsResponse {
            target: guid,
            lifetime_hk: 1,
            today_contribution: 2,
            yesterday_contribution: 3,
            today_hk: 4,
            yesterday_hk: 5,
            lifetime_max_rank: 6,
            honor_level: 7,
        }
        .to_bytes();

        let mut packet = WorldPacket::from_bytes(&bytes);
        assert_eq!(
            packet.read_uint16().expect("opcode"),
            ServerOpcodes::InspectHonorStats as u16
        );
        assert_eq!(packet.read_packed_guid().expect("target"), guid);
        assert_eq!(packet.read_uint32().expect("LifetimeHK"), 1);
        assert_eq!(packet.read_uint32().expect("TodayContribution"), 2);
        assert_eq!(packet.read_uint32().expect("YesterdayContribution"), 3);
        assert_eq!(packet.read_uint16().expect("TodayHK"), 4);
        assert_eq!(packet.read_uint16().expect("YesterdayHK"), 5);
        assert_eq!(packet.read_uint32().expect("LifetimeMaxRank"), 6);
        assert_eq!(packet.read_uint32().expect("HonorLevel"), 7);
    }
}
