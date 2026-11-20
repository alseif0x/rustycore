// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Inspect packet definitions: inspect request/response packets.

use crate::packets::misc::UpdateTalentData;
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
///
/// Layout as the 3.4.3.54261 client reads it (client reader at
/// `0x1406FF210`), which differs from TC 3.4.3 `InspectResult::Write` in two
/// places:
/// - there is no `Talents.Count` u32 between `Glyphs.Count` and `ItemLevel`;
/// - `Talents` is a full `TalentInfoUpdate` block (the same body as
///   SMSG_UPDATE_TALENT_DATA, client reader `0x140709E20`) written after the
///   glyph array, not a flat `u16` array.
pub struct InspectResult {
    pub target_guid: ObjectGuid,
    pub target_name: String,
    pub race: u8,
    pub class_id: u8,
    pub gender: u8,
    pub level: u32,
    pub items: Vec<InspectItem>,
    /// `TalentInfoUpdate` body, written exactly like SMSG_UPDATE_TALENT_DATA.
    pub talents: UpdateTalentData,
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
        // No Talents.Count here: the 54261 client reads ItemLevel next.
        w.write_int32(0); // ItemLevel
        w.write_uint8(0); // LifetimeMaxRank
        w.write_uint16(0); // TodayHK
        w.write_uint16(0); // YesterdayHK
        w.write_int32(0); // LifetimeHK
        w.write_int32(0); // HonorLevel
        // (no glyphs — loop empty)
        // Talents: TalentInfoUpdate, ending with the IsPetTalents bit, which
        // shares no byte with the following HasGuildData/HasAzeriteLevel bits.
        self.talents.write(w);
        w.flush_bits();
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

    /// Decodes SMSG_INSPECT_RESULT in the order of the 3.4.3.54261 client
    /// reader (`0x1406FF210`) and requires the packet to end exactly.
    #[test]
    fn inspect_result_matches_54261_client_reader() {
        use crate::packets::misc::{TalentGroupInfoLikeCpp, TalentInfoLikeCpp};

        let guid = ObjectGuid::create_player(1, 0x42);
        let mut group = TalentGroupInfoLikeCpp::default();
        group.spec_id = 3;
        group.talents = vec![
            TalentInfoLikeCpp {
                talent_id: 1001,
                rank: 2,
            },
            TalentInfoLikeCpp {
                talent_id: 1002,
                rank: 0,
            },
        ];
        group.glyph_ids = [11, 0, 22, 0, 0, 33];
        let bytes = InspectResult {
            target_guid: guid,
            target_name: "Abc".to_string(),
            race: 1,
            class_id: 2,
            gender: 1,
            level: 80,
            items: vec![InspectItem {
                slot: 4,
                item_id: 12345,
            }],
            talents: UpdateTalentData {
                unspent_talent_points: 7,
                active_group: 0,
                groups: vec![group],
                is_pet_talents: false,
            },
        }
        .to_bytes();

        let mut p = WorldPacket::from_bytes(&bytes);
        assert_eq!(
            p.read_uint16().expect("opcode"),
            ServerOpcodes::InspectResult as u16
        );

        // PlayerModelDisplayInfo
        assert_eq!(p.read_packed_guid().expect("GUID"), guid);
        assert_eq!(p.read_int32().expect("SpecID"), 0);
        assert_eq!(p.read_uint32().expect("Items.size"), 1);
        assert_eq!(p.read_bits(6).expect("name length"), 3);
        assert_eq!(p.read_uint8().expect("Gender"), 1);
        assert_eq!(p.read_uint8().expect("Race"), 1);
        assert_eq!(p.read_uint8().expect("Class"), 2);
        assert_eq!(p.read_uint32().expect("Customizations.size"), 0);
        assert_eq!(p.read_string(3).expect("name"), "Abc");

        // InspectItemData
        assert_eq!(p.read_packed_guid().expect("Creator"), ObjectGuid::EMPTY);
        assert_eq!(p.read_uint8().expect("Index"), 4);
        assert_eq!(p.read_uint32().expect("AzeritePowers"), 0);
        assert_eq!(p.read_uint32().expect("AzeriteEssences"), 0);
        assert_eq!(p.read_int32().expect("ItemID"), 12345);
        assert_eq!(p.read_int32().expect("RandomPropertiesSeed"), 0);
        assert_eq!(p.read_int32().expect("RandomPropertiesID"), 0);
        assert!(!p.read_bit().expect("HasItemBonus"));
        p.reset_bits();
        assert_eq!(p.read_bits(6).expect("ItemModList size"), 0);
        p.reset_bits();
        assert!(p.read_bit().expect("Usable"));
        assert_eq!(p.read_bits(4).expect("Enchants.size"), 0);
        assert_eq!(p.read_bits(2).expect("Gems.size"), 0);
        p.reset_bits();

        // No Talents.Count between Glyphs.Count and ItemLevel.
        assert_eq!(p.read_uint32().expect("Glyphs.size"), 0);
        assert_eq!(p.read_int32().expect("ItemLevel"), 0);
        assert_eq!(p.read_uint8().expect("LifetimeMaxRank"), 0);
        assert_eq!(p.read_uint16().expect("TodayHK"), 0);
        assert_eq!(p.read_uint16().expect("YesterdayHK"), 0);
        assert_eq!(p.read_uint32().expect("LifetimeHK"), 0);
        assert_eq!(p.read_uint32().expect("HonorLevel"), 0);

        // Talents: TalentInfoUpdate (SMSG_UPDATE_TALENT_DATA body).
        assert_eq!(p.read_uint32().expect("UnspentTalentPoints"), 7);
        assert_eq!(p.read_uint8().expect("ActiveGroup"), 0);
        assert_eq!(p.read_uint32().expect("Groups.size"), 1);
        assert_eq!(p.read_uint8().expect("NumTalents"), 2);
        assert_eq!(p.read_uint32().expect("Talents.size"), 2);
        assert_eq!(p.read_uint8().expect("NumGlyphs"), 6);
        assert_eq!(p.read_uint32().expect("Glyphs.size"), 6);
        assert_eq!(p.read_uint8().expect("SpecID"), 3);
        assert_eq!(p.read_uint32().expect("TalentID"), 1001);
        assert_eq!(p.read_uint8().expect("Rank"), 2);
        assert_eq!(p.read_uint32().expect("TalentID"), 1002);
        assert_eq!(p.read_uint8().expect("Rank"), 0);
        for expected in [11u16, 0, 22, 0, 0, 33] {
            assert_eq!(p.read_uint16().expect("glyph"), expected);
        }
        assert!(!p.read_bit().expect("IsPetTalents"));
        p.reset_bits();

        // HasGuildData, HasAzeriteLevel in their own byte.
        assert_eq!(p.read_uint8().expect("guild/azerite bits"), 0);

        // 7 x PVPBracketData: u8 Bracket, 17 x i32, byte with Disqualified.
        for bracket in 0u8..7 {
            assert_eq!(p.read_uint8().expect("Bracket"), bracket);
            for _ in 0..17 {
                assert_eq!(p.read_int32().expect("bracket field"), 0);
            }
            assert_eq!(p.read_uint8().expect("Disqualified"), 0);
        }

        // TraitInspectInfo with an empty TraitConfig (Type 0).
        assert_eq!(p.read_int32().expect("Level"), 80);
        assert_eq!(p.read_int32().expect("ChrSpecializationID"), 0);
        assert_eq!(p.read_int32().expect("TraitConfig.ID"), 0);
        assert_eq!(p.read_int32().expect("TraitConfig.Type"), 0);
        assert_eq!(p.read_uint32().expect("Entries.size"), 0);
        assert_eq!(p.read_bits(9).expect("name length"), 0);
        p.reset_bits();
        assert_eq!(p.remaining(), 0, "packet must end after TraitInspectInfo");
    }

    #[test]
    fn inspect_result_default_talents_have_exact_54261_size() {
        let guid = ObjectGuid::create_player(1, 0x7);
        let bytes = InspectResult {
            target_guid: guid,
            target_name: String::new(),
            race: 1,
            class_id: 1,
            gender: 0,
            level: 1,
            items: Vec::new(),
            talents: UpdateTalentData::default(),
        }
        .to_bytes();

        let mut guid_buf = WorldPacket::new_empty();
        guid_buf.write_packed_guid(&guid);
        let guid_len = guid_buf.into_data().len();
        // opcode 2; display info: guid, 4 + 4 + 1 (name bits) + 3 + 4;
        // 4 Glyphs.Count + 4 ItemLevel + 1 + 2 + 2 + 4 + 4 honor;
        // talents: 4 + 1 + 4 + one empty group (1 + 4 + 1 + 4 + 1 + 12) + 1;
        // 1 guild/azerite; 7 x (1 + 68 + 1); traits 4 + 4 + 4 + 4 + 4 + 2.
        let expected = 2 + guid_len + 16 + 21 + 33 + 1 + 7 * 70 + 22;
        assert_eq!(bytes.len(), expected);
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
