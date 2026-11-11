//! Frozen SQL setter inputs; canonical scalar conversions and eligibility remain absent.
pub struct NewCharacterStatistics {
    pub max_health: u32,
    pub powers: [u32; 10],
    pub stats: [u32; 5],
    pub resistances: [u32; 7],
    /// Block, dodge, parry, crit, ranged crit, spell crit.
    pub percentages: [f32; 6],
    pub attack_power: u32,
    pub ranged_attack_power: u32,
    pub spell_power: u32,
    pub resilience: u32,
    pub mastery: f32,
    pub versatility: i32,
}
