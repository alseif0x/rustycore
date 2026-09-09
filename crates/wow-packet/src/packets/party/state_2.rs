//! Party packets state definitions, part 2 of 2.
//!
//! Separated from the party.rs root under #650. Behaviour is preserved.

use super::*;

#[derive(Debug, Clone, PartialEq)]
pub struct PartyMemberAuraState {
    pub spell_id: i32,
    pub flags: u16,
    pub active_flags: u32,
    pub points: Vec<f32>,
}

impl PartyMemberAuraState {
    pub(super) fn write(&self, w: &mut WorldPacket) {
        w.write_int32(self.spell_id);
        w.write_uint16(self.flags);
        w.write_uint32(self.active_flags);
        w.write_int32(self.points.len() as i32);
        for point in &self.points {
            w.write_float(*point);
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PartyMemberPetStats {
    pub guid: ObjectGuid,
    pub model_id: i32,
    pub current_health: i32,
    pub max_health: i32,
    pub auras: Vec<PartyMemberAuraState>,
    pub name: String,
}

impl PartyMemberPetStats {
    pub(super) fn write(&self, w: &mut WorldPacket) {
        w.write_packed_guid(&self.guid);
        w.write_int32(self.model_id);
        w.write_int32(self.current_health);
        w.write_int32(self.max_health);
        w.write_uint32(self.auras.len() as u32);
        for aura in &self.auras {
            aura.write(w);
        }
        w.write_bits(self.name.len() as u32, 8);
        w.flush_bits();
        w.write_string(&self.name);
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DungeonScoreMapSummary {
    pub challenge_mode_id: i32,
    pub map_score: f32,
    pub best_run_level: i32,
    pub best_run_duration_ms: i32,
    pub finished_success: bool,
}

impl DungeonScoreMapSummary {
    pub(super) fn write(&self, w: &mut WorldPacket) {
        w.write_int32(self.challenge_mode_id);
        w.write_float(self.map_score);
        w.write_int32(self.best_run_level);
        w.write_int32(self.best_run_duration_ms);
        w.write_bit(self.finished_success);
        w.flush_bits();
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DungeonScoreSummary {
    pub overall_score_current_season: f32,
    pub ladder_score_current_season: f32,
    pub runs: Vec<DungeonScoreMapSummary>,
}

impl DungeonScoreSummary {
    pub(super) fn write(&self, w: &mut WorldPacket) {
        w.write_float(self.overall_score_current_season);
        w.write_float(self.ladder_score_current_season);
        w.write_uint32(self.runs.len() as u32);
        for run in &self.runs {
            run.write(w);
        }
    }
}

pub struct PartyMemberFullState {
    pub member_guid: ObjectGuid,
    pub for_enemy: bool,
    // Stats
    pub status: u16, // GroupMemberOnlineStatus: 0x0001=online
    pub power_type: u8,
    pub current_health: i32,
    pub max_health: i32,
    pub current_power: u16,
    pub max_power: u16,
    pub level: u16,
    pub spec_id: u16,
    pub zone_id: u16,
    pub position_x: i16,
    pub position_y: i16,
    pub position_z: i16,
    pub vehicle_seat: i32,
    pub party_type: [u8; 2],
    pub phases: PartyMemberPhaseStates,
    pub auras: Vec<PartyMemberAuraState>,
    pub pet_stats: Option<PartyMemberPetStats>,
    pub dungeon_score: DungeonScoreSummary,
}

impl ServerPacket for PartyMemberFullState {
    const OPCODE: ServerOpcodes = ServerOpcodes::PartyMemberFullState;
    fn write(&self, w: &mut WorldPacket) {
        w.write_bit(self.for_enemy);
        w.flush_bits();

        // PartyMemberStats.Write():
        w.write_uint8(self.party_type[0]);
        w.write_uint8(self.party_type[1]);
        w.write_int16(self.status as i16);
        w.write_uint8(self.power_type);
        w.write_int16(0); // PowerDisplayID
        w.write_int32(self.current_health);
        w.write_int32(self.max_health);
        w.write_uint16(self.current_power);
        w.write_uint16(self.max_power);
        w.write_uint16(self.level);
        w.write_uint16(self.spec_id);
        w.write_uint16(self.zone_id);
        w.write_uint16(0); // WmoGroupID
        w.write_uint32(0); // WmoDoodadPlacementID
        w.write_int16(self.position_x);
        w.write_int16(self.position_y);
        w.write_int16(self.position_z);
        w.write_int32(self.vehicle_seat);
        w.write_uint32(self.auras.len() as u32);

        self.phases.write(w);

        // CTROptions.Write() — empty:
        w.write_uint32(0); // ContentTuningConditionMask
        w.write_int32(0); // Unused901
        w.write_uint32(0); // ExpansionLevelMask

        for aura in &self.auras {
            aura.write(w);
        }

        w.write_bit(self.pet_stats.is_some()); // PetStats != null
        w.flush_bits();

        self.dungeon_score.write(w);

        if let Some(pet_stats) = &self.pet_stats {
            pet_stats.write(w);
        }

        w.write_packed_guid(&self.member_guid);
    }
}
