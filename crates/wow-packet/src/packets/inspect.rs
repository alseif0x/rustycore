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

/// CMSG_QUERY_INSPECT_ACHIEVEMENTS.
///
/// C++ `WorldPackets::Inspect::QueryInspectAchievements::Read` reads a target
/// `ObjectGuid`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueryInspectAchievements {
    pub guid: ObjectGuid,
}

impl ClientPacket for QueryInspectAchievements {
    const OPCODE: ClientOpcodes = ClientOpcodes::QueryInspectAchievements;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            guid: packet.read_packed_guid()?,
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
        // PlayerModelDisplayInfo: GUID, SpecializationID, display, Items.Count.
        w.write_packed_guid(&self.target_guid);
        w.write_int32(0);
        let name_bytes = self.target_name.as_bytes();
        w.write_bits(name_bytes.len() as u32, 6);
        w.write_uint8(self.gender);
        w.write_uint8(self.race);
        w.write_uint8(self.class_id);
        w.write_int32(0); // Customizations.Count
        w.write_bytes(name_bytes);
        w.write_int32(self.items.len() as i32);

        for item in &self.items {
            w.write_packed_guid(&ObjectGuid::EMPTY); // CreatorGUID
            // ItemInstance precedes InspectItemData.Index and Azerite counts.
            w.write_int32(item.item_id);
            w.write_bits(0u32, 7); // ItemModList.Values.Count
            w.flush_bits();
            w.write_bit(false); // ItemBonus != null
            w.flush_bits();
            w.write_uint8(item.slot); // Index
            w.write_int32(0); // AzeritePowers.Count
            w.write_int32(0); // AzeriteEssences.Count
            w.write_bit(true); // Usable
            w.write_bits(0u32, 4); // Enchants.Count
            w.write_bits(0u32, 2); // Gems.Count
            w.flush_bits();
        }

        w.write_int32(0); // Glyphs.Count
        w.write_int32(0); // Talents.Count
        w.write_int32(4); // PvpTalents.Count (MAX_PVP_TALENT_SLOTS)
        // Empty ClassicTalentInfoUpdate, including its trailing bit flush.
        w.write_int32(0); // UnspentTalentPoints
        w.write_uint8(0); // ActiveGroup
        w.write_int32(0); // TalentGroupInfos.Count
        w.write_bit(false); // IsPetTalents
        w.flush_bits();
        w.write_int32(0); // ItemLevel

        for bracket in 0u8..9 {
            w.write_uint8(bracket);
            w.write_int32(0); // RatingID
            w.write_int32(0); // Rating
            w.write_int32(0); // Rank
            w.write_int32(0); // WeeklyPlayed
            w.write_int32(0); // WeeklyWon
            w.write_int32(0); // SeasonPlayed
            w.write_int32(0); // SeasonWon
            w.write_int32(0); // WeeklyBestRating
            w.write_int32(0); // LastWeeksBestRating
            w.write_int32(0); // Tier
            w.write_int32(0); // WeeklyBestTier
            w.write_int32(0); // SeasonBestRating
            w.write_uint8(0); // SeasonBestTierEnum
            w.write_int32(0); // RoundsSeasonPlayed
            w.write_int32(0); // RoundsSeasonWon
            w.write_int32(0); // RoundsWeeklyPlayed
            w.write_int32(0); // RoundsWeeklyWon
            w.write_bit(false); // Disqualified
            w.flush_bits();
        }

        w.write_uint8(0); // LifetimeMaxRank
        w.write_uint16(0); // TodayHK
        w.write_uint16(0); // YesterdayHK
        w.write_int32(0); // LifetimeHK
        w.write_int32(0); // HonorLevel
        // TraitInspectInfo and its empty TraitConfig.
        w.write_int32(self.level as i32);
        w.write_int32(0); // ChrSpecializationID
        w.write_int32(0); // TraitConfig.ID
        w.write_int32(0); // TraitConfig.Type
        w.write_int32(0); // TraitConfig.Entries.Count
        w.write_bits(0u32, 9); // TraitConfig.Name length
        w.flush_bits();
        // Glyphs and Talents are empty; PvpTalents always has four slots.
        for _ in 0..4 {
            w.write_uint16(0);
        }
        w.write_bit(false); // GuildData.HasValue
        w.write_bit(false); // AzeriteLevel.HasValue
        w.flush_bits();
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

/// SMSG_RESPOND_INSPECT_ACHIEVEMENTS.
///
/// C++ writes `Player` followed by `AllAchievements`: earned count, progress
/// count, then both arrays. Rust represents the empty achievement-manager
/// branch until `PlayerAchievementMgr` is ported.
pub struct RespondInspectAchievements {
    pub player: ObjectGuid,
}

impl ServerPacket for RespondInspectAchievements {
    const OPCODE: ServerOpcodes = ServerOpcodes::RespondInspectAchievements;

    fn write(&self, w: &mut WorldPacket) {
        w.write_packed_guid(&self.player);
        w.write_uint32(0); // Earned.Count
        w.write_uint32(0); // Progress.Count
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
    fn query_inspect_achievements_reads_target_guid_like_cpp() {
        let guid = ObjectGuid::create_player(1, 0x5678);
        let mut writer = WorldPacket::new_empty();
        writer.write_packed_guid(&guid);

        let mut packet = WorldPacket::from_bytes(&writer.into_data());
        let parsed =
            QueryInspectAchievements::read(&mut packet).expect("query inspect achievements");

        assert_eq!(parsed.guid, guid);
    }

    #[test]
    fn inspect_result_writes_forever_cpp_field_order() {
        let guid = ObjectGuid::create_player(1, 0x1234);
        let bytes = InspectResult {
            target_guid: guid,
            target_name: "Misha".into(),
            race: 1,
            class_id: 2,
            gender: 1,
            level: 42,
            items: vec![InspectItem {
                slot: 7,
                item_id: 12_345,
            }],
        }
        .to_bytes();
        let mut packet = WorldPacket::from_bytes(&bytes);
        assert_eq!(
            packet.read_uint16().unwrap(),
            ServerOpcodes::InspectResult as u16
        );
        assert_eq!(packet.read_packed_guid().unwrap(), guid);
        assert_eq!(packet.read_int32().unwrap(), 0); // SpecializationID
        assert_eq!(packet.read_bits(6).unwrap(), 5);
        assert_eq!(packet.read_uint8().unwrap(), 1); // Gender
        assert_eq!(packet.read_uint8().unwrap(), 1); // Race
        assert_eq!(packet.read_uint8().unwrap(), 2); // Class
        assert_eq!(packet.read_int32().unwrap(), 0); // Customizations.Count
        assert_eq!(packet.read_string(5).unwrap(), "Misha");
        assert_eq!(packet.read_int32().unwrap(), 1); // Items.Count
        assert_eq!(packet.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
        assert_eq!(packet.read_int32().unwrap(), 12_345); // ItemID
        assert_eq!(packet.read_bits(7).unwrap(), 0); // ItemModList.Values.Count
        packet.reset_bits();
        assert!(!packet.read_bit().unwrap()); // ItemBonus
        packet.reset_bits();
        assert_eq!(packet.read_uint8().unwrap(), 7); // Index
        assert_eq!(packet.read_int32().unwrap(), 0); // AzeritePowers.Count
        assert_eq!(packet.read_int32().unwrap(), 0); // AzeriteEssences.Count
        assert!(packet.read_bit().unwrap()); // Usable
        assert_eq!(packet.read_bits(4).unwrap(), 0); // Enchants.Count
        assert_eq!(packet.read_bits(2).unwrap(), 0); // Gems.Count
        packet.reset_bits();
        assert_eq!(packet.read_int32().unwrap(), 0); // Glyphs.Count
        assert_eq!(packet.read_int32().unwrap(), 0); // Talents.Count
        assert_eq!(packet.read_int32().unwrap(), 4); // PvpTalents.Count
        assert_eq!(packet.read_int32().unwrap(), 0); // UnspentTalentPoints
        assert_eq!(packet.read_uint8().unwrap(), 0); // ActiveGroup
        assert_eq!(packet.read_int32().unwrap(), 0); // TalentGroupInfos.Count
        assert!(!packet.read_bit().unwrap()); // IsPetTalents
        packet.reset_bits();
        assert_eq!(packet.read_int32().unwrap(), 0); // ItemLevel
        for bracket in 0u8..9 {
            assert_eq!(packet.read_uint8().unwrap(), bracket);
            for _ in 0..12 {
                assert_eq!(packet.read_int32().unwrap(), 0);
            }
            assert_eq!(packet.read_uint8().unwrap(), 0); // SeasonBestTierEnum
            for _ in 0..4 {
                assert_eq!(packet.read_int32().unwrap(), 0);
            }
            assert!(!packet.read_bit().unwrap()); // Disqualified
            packet.reset_bits();
        }
        assert_eq!(packet.read_uint8().unwrap(), 0); // LifetimeMaxRank
        assert_eq!(packet.read_uint16().unwrap(), 0); // TodayHK
        assert_eq!(packet.read_uint16().unwrap(), 0); // YesterdayHK
        assert_eq!(packet.read_int32().unwrap(), 0); // LifetimeHK
        assert_eq!(packet.read_int32().unwrap(), 0); // HonorLevel
        assert_eq!(packet.read_int32().unwrap(), 42); // TraitInspectInfo.Level
        assert_eq!(packet.read_int32().unwrap(), 0); // ChrSpecializationID
        assert_eq!(packet.read_int32().unwrap(), 0); // TraitConfig.ID
        assert_eq!(packet.read_int32().unwrap(), 0); // TraitConfig.Type
        assert_eq!(packet.read_int32().unwrap(), 0); // TraitConfig.Entries.Count
        assert_eq!(packet.read_bits(9).unwrap(), 0); // TraitConfig.Name length
        packet.reset_bits();
        for _ in 0..4 {
            assert_eq!(packet.read_uint16().unwrap(), 0); // PvpTalents slot
        }
        assert!(!packet.read_bit().unwrap()); // GuildData.HasValue
        assert!(!packet.read_bit().unwrap()); // AzeriteLevel.HasValue
        packet.reset_bits();
        assert!(
            packet.read_uint8().is_err(),
            "packet must be fully consumed"
        );
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

    #[test]
    fn respond_inspect_achievements_writes_empty_all_achievements_like_cpp() {
        let guid = ObjectGuid::create_player(1, 0x99);
        let bytes = RespondInspectAchievements { player: guid }.to_bytes();

        let mut packet = WorldPacket::from_bytes(&bytes);
        assert_eq!(
            packet.read_uint16().expect("opcode"),
            ServerOpcodes::RespondInspectAchievements as u16
        );
        assert_eq!(packet.read_packed_guid().expect("player"), guid);
        assert_eq!(packet.read_uint32().expect("Earned.Count"), 0);
        assert_eq!(packet.read_uint32().expect("Progress.Count"), 0);
    }
}
