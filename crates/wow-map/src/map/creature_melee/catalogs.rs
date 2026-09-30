use super::*;
use std::collections::HashMap;
use wow_entities::{AppliedAuraRef, AuraApplicationLikeCpp, AuraSubsystem};

/// Borrowed catalog queries for this synchronous operation only. No method
/// accepts a manager, map, session, packet sink or mutable entity.
pub trait CreatureMeleeCatalogsLikeCpp {
    fn represented(&self) -> bool;
    fn creature_effects(&self, applied: &[AppliedAuraRef], difficulty: u8)
        -> Vec<AppliedAuraEffectLikeCpp>;
    fn player_effects(&self, auras: &HashMap<u8, AuraApplicationLikeCpp>)
        -> Vec<AppliedAuraEffectLikeCpp>;
    fn player_effects_of_type(&self, auras: &HashMap<u8, AuraApplicationLikeCpp>, aura_type: i32)
        -> Vec<AppliedAuraEffectLikeCpp>;
    fn player_amounts_of_type(&self, auras: &HashMap<u8, AuraApplicationLikeCpp>, aura_type: i32)
        -> Vec<(i32, i32)>;
    fn player_mechanic_mask(&self, auras: &HashMap<u8, AuraApplicationLikeCpp>) -> u32;
    fn creature_mechanic_mask(&self, applied: &[AppliedAuraRef], difficulty: u8) -> u32;
    fn race_creature_type_mask(&self, race: u8) -> u32;
    fn template_creature_type_mask(&self, entry: u32) -> u32;
    fn block_armor_constant(&self, attacker_level: u8) -> f32;
    fn player_shields(&self, auras: &HashMap<u8, AuraApplicationLikeCpp>, difficulty: u8,
        school_mask: u32) -> Vec<RepresentedAbsorbShieldLikeCpp>;
    fn player_mana_shields(&self, auras: &HashMap<u8, AuraApplicationLikeCpp>, difficulty: u8,
        school_mask: u32) -> Vec<RepresentedManaShieldLikeCpp>;
    fn creature_shields(&self, auras: &AuraSubsystem, difficulty: u8,
        school_mask: u32) -> Vec<RepresentedAbsorbShieldLikeCpp>;
    fn share_player(&self, auras: &HashMap<u8, AuraApplicationLikeCpp>)
        -> Vec<ShareAuraSnapshotLikeCpp>;
    fn share_creature(&self, applied: &[AppliedAuraRef], difficulty: u8)
        -> Vec<ShareAuraSnapshotLikeCpp>;
    fn threat_spell(&self, spell_id: Option<i32>, difficulty: u8) -> MeleeThreatSpellFacts;
    fn threat_aura(&self, applied: &[AppliedAuraRef], difficulty: u8, school_mask: u32) -> f32;
    fn game_time_ms(&self) -> u64;
}
