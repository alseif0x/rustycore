//! SQL-free glyph, normal-talent and PvP-talent save projections.

/// Five fixed specialization groups. Glyph and normal-talent vectors preserve
/// source iteration order; construction must supply every group explicitly.
pub struct NewSpecializations {
    pub glyphs: [Vec<u32>; 5],
    pub talents: [Vec<NewTalent>; 5],
    pub pvp: [[u32; 4]; 5],
}

pub struct NewTalent {
    pub id: u32,
    pub removed: bool,
}
