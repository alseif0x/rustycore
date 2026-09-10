//! Creature packets.
//!
//! Separated from query.rs under #689.

use super::*;

/// C++ `MAX_CREATURE_NAMES` (`Entities/Creature/CreatureData.h:442`).
const MAX_CREATURE_NAMES: usize = 4;

/// Maximum creature kill credit slots.
const MAX_CREATURE_KILL_CREDIT: usize = 2;

// ── CMSG_QUERY_CREATURE (0x3270) ─────────────────────────────────────

/// Client request for creature template data.
pub struct QueryCreature {
    pub creature_id: u32,
}

impl ClientPacket for QueryCreature {
    const OPCODE: ClientOpcodes = ClientOpcodes::QueryCreature;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        let creature_id = packet.read_uint32()?;
        Ok(Self { creature_id })
    }
}

// ── SMSG_CREATURE_QUERY_RESPONSE (0x2914) ────────────────────────────

/// A single display variant for a creature.
pub struct CreatureXDisplay {
    pub creature_display_id: u32,
    pub scale: f32,
    pub probability: f32,
}

/// Creature display info block.
pub struct CreatureDisplayStats {
    pub displays: Vec<CreatureXDisplay>,
    pub total_probability: f32,
}

/// Full creature template stats for query response.
pub struct CreatureStats {
    pub title: String, // SubName in DB
    pub title_alt: String,
    pub cursor_name: String, // IconName in DB
    pub civilian: bool,
    pub leader: bool, // RacialLeader
    pub names: [String; MAX_CREATURE_NAMES],
    pub name_alts: [String; MAX_CREATURE_NAMES],
    pub flags: [u32; 2], // TypeFlags, TypeFlags2
    pub creature_type: i32,
    pub creature_family: i32,
    pub classification: i32,
    pub proxy_creature_ids: [i32; MAX_CREATURE_KILL_CREDIT],
    pub display: CreatureDisplayStats,
    pub hp_multi: f32,
    pub energy_multi: f32,
    pub quest_items: Vec<i32>,
    pub creature_movement_info_id: i32,
    pub health_scaling_expansion: i32,
    pub required_expansion: i32,
    pub vignette_id: i32,
    pub unit_class: i32,
    pub creature_difficulty_id: i32,
    pub widget_set_id: i32,
    pub widget_set_unit_condition_id: i32,
}

impl Default for CreatureStats {
    fn default() -> Self {
        Self {
            title: String::new(),
            title_alt: String::new(),
            cursor_name: String::new(),
            civilian: false,
            leader: false,
            names: Default::default(),
            name_alts: Default::default(),
            flags: [0; 2],
            creature_type: 0,
            creature_family: 0,
            classification: 0,
            proxy_creature_ids: [0; MAX_CREATURE_KILL_CREDIT],
            display: CreatureDisplayStats {
                displays: Vec::new(),
                total_probability: 0.0,
            },
            hp_multi: 1.0,
            energy_multi: 1.0,
            quest_items: Vec::new(),
            creature_movement_info_id: 0,
            health_scaling_expansion: 0,
            required_expansion: 0,
            vignette_id: 0,
            unit_class: 1,
            creature_difficulty_id: 0,
            widget_set_id: 0,
            widget_set_unit_condition_id: 0,
        }
    }
}

/// Server response with creature template data.
pub struct QueryCreatureResponse {
    pub creature_id: u32,
    pub allow: bool,
    pub stats: Option<CreatureStats>,
}

impl ServerPacket for QueryCreatureResponse {
    const OPCODE: ServerOpcodes = ServerOpcodes::QueryCreatureResponse;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.creature_id as i32);
        pkt.write_bit(self.allow);
        pkt.flush_bits();

        if !self.allow {
            return;
        }

        let default_stats;
        let stats = match &self.stats {
            Some(s) => s,
            None => {
                default_stats = CreatureStats::default();
                &default_stats
            }
        };

        // ── Bit-packed string lengths ────────────────────────────
        // C++ writes length() + 1 for every nullable string length bitfield,
        // including empty strings, but only emits string bytes when non-empty.
        let title_len = stats.title.len() as u32 + 1;
        let title_alt_len = stats.title_alt.len() as u32 + 1;
        let cursor_name_len = stats.cursor_name.len() as u32 + 1;

        pkt.write_bits(title_len, 11);
        pkt.write_bits(title_alt_len, 11);
        pkt.write_bits(cursor_name_len, 6);
        pkt.write_bit(stats.civilian);
        pkt.write_bit(stats.leader);

        // C++ `QueryCreatureResponse::Write` interleaves Name[i] and NameAlt[i]
        // lengths in one loop (`Server/Packets/QueryPackets.cpp:83-87`).
        for i in 0..MAX_CREATURE_NAMES {
            let name_len = stats.names[i].len() as u32 + 1;
            let alt_len = stats.name_alts[i].len() as u32 + 1;
            pkt.write_bits(name_len, 11);
            pkt.write_bits(alt_len, 11);
        }
        pkt.flush_bits();

        // ── Name strings (BEFORE integer fields!) ────────────────
        // C++ writes names interleaved: Name[0], NameAlt[0], Name[1], NameAlt[1], ...
        // (`Server/Packets/QueryPackets.cpp:91-98`).
        for i in 0..MAX_CREATURE_NAMES {
            if !stats.names[i].is_empty() {
                pkt.write_cstring(&stats.names[i]);
            }
            if !stats.name_alts[i].is_empty() {
                pkt.write_cstring(&stats.name_alts[i]);
            }
        }

        // ── Integer fields ───────────────────────────────────────
        // Flags[2]
        pkt.write_uint32(stats.flags[0]);
        pkt.write_uint32(stats.flags[1]);

        // Type, Family, Classification
        pkt.write_int32(stats.creature_type);
        pkt.write_int32(stats.creature_family);
        pkt.write_int32(stats.classification);

        // PetSpellDataId — not used in 3.4.3, always 0
        pkt.write_int32(0);

        // ProxyCreatureID (kill credits)
        for i in 0..MAX_CREATURE_KILL_CREDIT {
            pkt.write_int32(stats.proxy_creature_ids[i]);
        }

        // Display info
        pkt.write_int32(stats.display.displays.len() as i32);
        pkt.write_float(stats.display.total_probability);

        for d in &stats.display.displays {
            pkt.write_int32(d.creature_display_id as i32);
            pkt.write_float(d.scale);
            pkt.write_float(d.probability);
        }

        // Multipliers
        pkt.write_float(stats.hp_multi);
        pkt.write_float(stats.energy_multi);

        // Quest items count
        pkt.write_int32(stats.quest_items.len() as i32);

        // Remaining integer fields
        pkt.write_int32(stats.creature_movement_info_id);
        pkt.write_int32(stats.health_scaling_expansion);
        pkt.write_int32(stats.required_expansion);
        pkt.write_int32(stats.vignette_id);
        pkt.write_int32(stats.unit_class);
        pkt.write_int32(stats.creature_difficulty_id);
        pkt.write_int32(stats.widget_set_id);
        pkt.write_int32(stats.widget_set_unit_condition_id);

        // ── Trailing strings ─────────────────────────────────────
        if !stats.title.is_empty() {
            pkt.write_cstring(&stats.title);
        }
        if !stats.title_alt.is_empty() {
            pkt.write_cstring(&stats.title_alt);
        }
        if !stats.cursor_name.is_empty() {
            pkt.write_cstring(&stats.cursor_name);
        }

        // Quest item IDs
        for &item_id in &stats.quest_items {
            pkt.write_int32(item_id);
        }
    }
}
