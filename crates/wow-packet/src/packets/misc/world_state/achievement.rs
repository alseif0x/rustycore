//! AchievementPackets.cpp, client build 54261.
use super::*;

#[derive(Clone, Copy, Debug)]
pub struct AchievementProgress {
    pub id: u32,
    pub quantity: u64,
    pub player: ObjectGuid,
    pub packed_date: u32,
}

impl AchievementProgress {
    fn write_through_elapsed(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.id);
        pkt.write_uint64(self.quantity);
        pkt.write_packed_guid(&self.player);
        pkt.write_uint32(0); // Unused_10_1_5
        pkt.write_uint32(0); // Untimed criterion flags
        pkt.write_uint32(self.packed_date);
        pkt.write_int64(0); // Duration<Seconds, int64>
    }
}

pub struct CriteriaUpdate(pub AchievementProgress);

impl ServerPacket for CriteriaUpdate {
    const OPCODE: ServerOpcodes = ServerOpcodes::CriteriaUpdate;

    fn write(&self, pkt: &mut WorldPacket) {
        self.0.write_through_elapsed(pkt);
        pkt.write_int64(0); // CriteriaUpdate::CreationTime is Timestamp<int64>.
        pkt.write_bit(false);
        pkt.flush_bits();
    }
}

#[derive(Clone, Copy, Debug)]
pub struct EarnedAchievement {
    pub id: u32,
    pub packed_date: u32,
    pub owner: ObjectGuid,
    pub realm: u32,
}

#[derive(Default)]
pub struct AllAchievementData {
    pub earned: Vec<EarnedAchievement>,
    pub progress: Vec<AchievementProgress>,
}

impl ServerPacket for AllAchievementData {
    const OPCODE: ServerOpcodes = ServerOpcodes::AllAchievementData;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.earned.len() as u32);
        pkt.write_uint32(self.progress.len() as u32);
        for earned in &self.earned {
            pkt.write_uint32(earned.id);
            pkt.write_uint32(earned.packed_date);
            pkt.write_packed_guid(&earned.owner);
            pkt.write_uint32(earned.realm);
            pkt.write_uint32(earned.realm);
        }
        for progress in &self.progress {
            progress.write_through_elapsed(pkt);
            pkt.write_int64(0); // TimeFromCreate
            pkt.write_bit(false);
            pkt.flush_bits();
        }
    }
}

pub struct AchievementEarned(pub EarnedAchievement);

impl ServerPacket for AchievementEarned {
    const OPCODE: ServerOpcodes = ServerOpcodes::AchievementEarned;

    fn write(&self, pkt: &mut WorldPacket) {
        let earned = &self.0;
        pkt.write_packed_guid(&earned.owner);
        pkt.write_packed_guid(&earned.owner);
        pkt.write_uint32(earned.id);
        pkt.write_uint32(earned.packed_date);
        pkt.write_uint32(earned.realm);
        pkt.write_uint32(earned.realm);
        pkt.write_bit(false);
        pkt.flush_bits();
    }
}
