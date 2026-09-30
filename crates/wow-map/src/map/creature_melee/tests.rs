//! Direct motor contracts over canonical entities and resolved catalog facts.
use super::*;
use std::cell::RefCell;
use std::collections::HashMap;
use rand::{Rng, RngCore, SeedableRng, rngs::StdRng};
use wow_core::guid::HighGuid;
use wow_entities::{AppliedAuraRef, AuraApplicationLikeCpp, AuraSubsystem,
    CreatureAvoidanceLikeCpp, MapObjectRecord};
use wow_constants::spell::aura_types::*;

mod readiness;
mod settlement;
mod sources;
mod los_delegate;

#[derive(Default)]
struct Catalogs {
    represented: bool,
    calls: RefCell<Vec<&'static str>>,
    effects: Vec<AppliedAuraEffectLikeCpp>,
    school: Vec<RepresentedAbsorbShieldLikeCpp>,
    mana: Vec<RepresentedManaShieldLikeCpp>,
    shares: Vec<ShareAuraSnapshotLikeCpp>,
}
impl Catalogs {
    fn inert() -> Self {
        Self { represented: true, effects: vec![
            effect(SPELL_AURA_MOD_HIT_CHANCE, 100, ObjectGuid::EMPTY),
            effect(SPELL_AURA_MOD_CRIT_PCT, -100, ObjectGuid::EMPTY),
            effect(SPELL_AURA_MOD_ENEMY_DODGE, 100, ObjectGuid::EMPTY),
        ], ..Default::default() }
    }
}
fn effect(aura_type: i32, amount: i32, caster_guid: ObjectGuid) -> AppliedAuraEffectLikeCpp {
    AppliedAuraEffectLikeCpp { slot: 3, spell_id: 42, caster_guid,
        aura_type, misc_value: 1, misc_value_b: 0, amount }
}
impl CreatureMeleeCatalogsLikeCpp for Catalogs {
    fn represented(&self) -> bool { self.represented }
    fn creature_effects(&self, _: &[AppliedAuraRef], _: u8) -> Vec<AppliedAuraEffectLikeCpp> {
        self.calls.borrow_mut().push("creature_effects"); self.effects.clone()
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
        -> Vec<RepresentedAbsorbShieldLikeCpp> {
        self.calls.borrow_mut().push("school"); self.school.clone()
    }
    fn player_mana_shields(&self, auras: &HashMap<u8, AuraApplicationLikeCpp>, _: u8, _: u32)
        -> Vec<RepresentedManaShieldLikeCpp> {
        self.calls.borrow_mut().push("mana");
        if !self.school.is_empty() {
            // The catalog receives the ORIGINAL snapshot even after school depletion.
            assert_eq!(auras[&1].represented_amount, 4);
        }
        self.mana.clone()
    }
    fn creature_shields(&self, _: &AuraSubsystem, _: u8, _: u32) -> Vec<RepresentedAbsorbShieldLikeCpp> {
        self.calls.borrow_mut().push("school"); self.school.clone()
    }
    fn share_player(&self, _: &HashMap<u8, AuraApplicationLikeCpp>) -> Vec<ShareAuraSnapshotLikeCpp> {
        self.calls.borrow_mut().push("share"); self.shares.clone()
    }
    fn share_creature(&self, _: &[AppliedAuraRef], _: u8) -> Vec<ShareAuraSnapshotLikeCpp> {
        self.calls.borrow_mut().push("share"); self.shares.clone()
    }
    fn threat_spell(&self, _: Option<i32>, _: u8) -> MeleeThreatSpellFacts {
        self.calls.borrow_mut().push("threat");
        MeleeThreatSpellFacts { suppress: false, no_initial_threat: false, school_mask: 1, multiplier: 1.0 }
    }
    fn threat_aura(&self, _: &[AppliedAuraRef], _: u8, _: u32) -> f32 {
        self.calls.borrow_mut().push("threat_aura"); 1.0
    }
    fn game_time_ms(&self) -> u64 { self.calls.borrow_mut().push("damage_clock"); 123 }
}

fn guid(counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, counter)
}
fn creature(counter: i64, position: Position, health: u64) -> Creature {
    let mut c = Creature::new(false);
    c.unit_mut().world_mut().object_mut().create(guid(counter));
    c.unit_mut().world_mut().object_mut().set_entry(1);
    c.unit_mut().world_mut().set_map(0, 0).unwrap();
    c.unit_mut().world_mut().relocate(position);
    c.unit_mut().world_mut().object_mut().add_to_world();
    c.unit_mut().set_level(80);
    c.unit_mut().set_max_health(100);
    c.unit_mut().set_health(health);
    c.set_ai_identity_runtime(1, 35, 0, 0);
    c.set_avoidance_like_cpp(CreatureAvoidanceLikeCpp { dodge_pct: 0.0, parry_pct: 0.0, block_pct: 0.0 });
    c
}
fn setup(victim_is_player: bool, distance: f32) -> (MapManager, WorldCreature, PendingCreatureSwingLikeCpp) {
    let mut manager = MapManager::default();
    let map = manager.create_world_map(0, 0).map_mut();
    let victim = if victim_is_player { ObjectGuid::create_player(1, 2) } else { guid(2) };
    if victim_is_player {
        let mut player = Player::new(Some(1), false);
        player.unit_mut().world_mut().object_mut().create(victim);
        player.unit_mut().world_mut().set_map(0, 0).unwrap();
        player.unit_mut().world_mut().relocate(Position::xyz(distance, 0.0, 0.0));
        player.unit_mut().world_mut().object_mut().add_to_world();
        player.unit_mut().set_level(80);
        player.unit_mut().set_max_health(100);
        player.unit_mut().set_health(100);
        map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap()).unwrap();
    } else {
        map.insert_map_object_record(MapObjectRecord::new_creature(
            creature(2, Position::xyz(distance, 0.0, 0.0), 100)).unwrap()).unwrap();
    }
    let c = creature(1, Position::ZERO, 100);
    let data = WorldCreature::create_data_from_canonical_like_cpp(&c);
    let mut attacker = WorldCreature::from_canonical(c, data);
    attacker.enter_combat(victim);
    attacker.creature.ai_ownership_mut().min_damage = 10;
    attacker.creature.ai_ownership_mut().max_damage = 10;
    attacker.creature.ai_ownership_mut().swing_timer_ms = 0;
    attacker.seed_runtime_rng_like_cpp(17);
    // This is the existing compatibility RECORD snapshot, not a cloned actor/motor.
    map.insert_map_object_record(MapObjectRecord::new_creature(attacker.creature.clone()).unwrap()).unwrap();
    let CreatureMeleeReadiness::Ready(swing) = creature_melee_readiness(&attacker, 0, 0) else { panic!("ready fixture"); };
    (manager, attacker, swing)
}
fn health(manager: &MapManager, guid: ObjectGuid) -> u64 {
    let map = manager.find_map(0, 0).unwrap().map();
    if guid.is_player() { map.get_typed_player(guid).unwrap().unit().data().health }
    else { map.with_creature_like_cpp(guid, |c| c.unit().data().health).unwrap() }
}
fn application(slot: u8, amount: i32) -> AuraApplicationLikeCpp {
    AuraApplicationLikeCpp { spell_id: 42, difficulty_id: 0, caster_guid: ObjectGuid::EMPTY,
        slot, duration_total: 0, duration_remaining: 0, stack_count: 1, aura_flags: 0,
        effect_mask: 1, aura_interrupt_flags: 0, aura_interrupt_flags2: 0,
        represented_effect: None, represented_amount: amount,
        represented_effect_amounts: vec![wow_entities::RepresentedAuraEffectAmountLikeCpp { effect_index: 0, amount }],
        represented_misc_value: None, represented_multiplier: 1.0,
        applied_at: std::time::Instant::now() }
}
