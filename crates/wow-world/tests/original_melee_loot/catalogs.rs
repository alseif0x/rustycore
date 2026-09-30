//! Existing real admission/grid/token builders and resolved immutable catalog rows.
use std::{cell::RefCell, collections::HashMap};
use wow_core::ObjectGuid;
use wow_map::map::CreatureMeleeCatalogsLikeCpp;
use wow_map::map::{MeleeThreatSpellFacts,
    ShareAuraSnapshotLikeCpp};
use wow_combat::{AppliedAuraEffectLikeCpp, RepresentedAbsorbShieldLikeCpp,
    RepresentedManaShieldLikeCpp};
use wow_entities::{AppliedAuraRef, AuraApplicationLikeCpp, AuraSubsystem};

#[derive(Default)]
pub(super) struct Catalogs {
    pub represented: bool,
    pub calls: RefCell<Vec<&'static str>>,
    pub effects: Vec<AppliedAuraEffectLikeCpp>,
    pub shares: Vec<ShareAuraSnapshotLikeCpp>,
}
impl CreatureMeleeCatalogsLikeCpp for Catalogs {
    fn represented(&self) -> bool { self.calls.borrow_mut().push("represented"); self.represented }
    fn creature_effects(&self, _: &[AppliedAuraRef], _: u8) -> Vec<AppliedAuraEffectLikeCpp> {
        self.calls.borrow_mut().push("effects"); self.effects.clone()
    }
    fn player_effects(&self, _: &HashMap<u8, AuraApplicationLikeCpp>) -> Vec<AppliedAuraEffectLikeCpp> {
        self.effects.clone()
    }
    fn player_effects_of_type(&self, _: &HashMap<u8, AuraApplicationLikeCpp>, t: i32)
        -> Vec<AppliedAuraEffectLikeCpp> {
        self.effects.iter().filter(|e| e.aura_type == t).copied().collect()
    }
    fn player_amounts_of_type(&self, _: &HashMap<u8, AuraApplicationLikeCpp>, t: i32) -> Vec<(i32,i32)> {
        self.effects.iter().filter(|e| e.aura_type == t).map(|e| (e.misc_value,e.amount)).collect()
    }
    fn player_mechanic_mask(&self, _: &HashMap<u8, AuraApplicationLikeCpp>) -> u32 { 0 }
    fn creature_mechanic_mask(&self, _: &[AppliedAuraRef], _: u8) -> u32 { 0 }
    fn race_creature_type_mask(&self, _: u8) -> u32 { 0 }
    fn template_creature_type_mask(&self, _: u32) -> u32 { 0 }
    fn block_armor_constant(&self, _: u8) -> f32 { 1.0 }
    fn player_shields(&self, _: &HashMap<u8, AuraApplicationLikeCpp>, _: u8, _: u32)
        -> Vec<RepresentedAbsorbShieldLikeCpp> { Vec::new() }
    fn player_mana_shields(&self, _: &HashMap<u8, AuraApplicationLikeCpp>, _: u8, _: u32)
        -> Vec<RepresentedManaShieldLikeCpp> { Vec::new() }
    fn creature_shields(&self, _: &AuraSubsystem, _: u8, _: u32)
        -> Vec<RepresentedAbsorbShieldLikeCpp> { Vec::new() }
    fn share_player(&self, _: &HashMap<u8, AuraApplicationLikeCpp>) -> Vec<ShareAuraSnapshotLikeCpp> {
        self.shares.clone()
    }
    fn share_creature(&self, _: &[AppliedAuraRef], _: u8) -> Vec<ShareAuraSnapshotLikeCpp> {
        self.calls.borrow_mut().push("share"); self.shares.clone()
    }
    fn threat_spell(&self, _: Option<i32>, _: u8) -> MeleeThreatSpellFacts {
        self.calls.borrow_mut().push("threat");
        MeleeThreatSpellFacts { suppress: false, no_initial_threat: false,
            school_mask: 1, multiplier: 1.0 }
    }
    fn threat_aura(&self, _: &[AppliedAuraRef], _: u8, _: u32) -> f32 { 1.0 }
    fn game_time_ms(&self) -> u64 { self.calls.borrow_mut().push("clock"); 123 }
}

