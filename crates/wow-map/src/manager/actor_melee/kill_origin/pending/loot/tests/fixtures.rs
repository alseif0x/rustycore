//! Existing real admission/grid/token builders and resolved immutable catalog rows.
use super::*;
use crate::manager::actor_melee::kill_origin::{
    CapturedMeleeKill, MeleeKillCapture, MeleeKillOccurrence,
};
use crate::manager::actor_tick_access::fixtures::*;
use crate::map::CreatureMeleeCatalogsLikeCpp;
use crate::map::{CreatureActorAdmission, MeleeThreatSpellFacts, ShareAuraSnapshotLikeCpp};
use std::{cell::RefCell, collections::HashMap};
use wow_combat::{
    AppliedAuraEffectLikeCpp, RepresentedAbsorbShieldLikeCpp, RepresentedManaShieldLikeCpp,
};
use wow_entities::{AppliedAuraRef, AuraApplicationLikeCpp, AuraSubsystem, MapObjectRecord};

#[derive(Default)]
pub(super) struct Catalogs {
    pub represented: bool,
    pub calls: RefCell<Vec<&'static str>>,
    pub effects: Vec<AppliedAuraEffectLikeCpp>,
    pub shares: Vec<ShareAuraSnapshotLikeCpp>,
}
impl CreatureMeleeCatalogsLikeCpp for Catalogs {
    fn represented(&self) -> bool {
        self.calls.borrow_mut().push("represented");
        self.represented
    }
    fn creature_effects(&self, _: &[AppliedAuraRef], _: u8) -> Vec<AppliedAuraEffectLikeCpp> {
        self.calls.borrow_mut().push("effects");
        self.effects.clone()
    }
    fn player_effects(
        &self,
        _: &HashMap<u8, AuraApplicationLikeCpp>,
    ) -> Vec<AppliedAuraEffectLikeCpp> {
        self.effects.clone()
    }
    fn player_effects_of_type(
        &self,
        _: &HashMap<u8, AuraApplicationLikeCpp>,
        t: i32,
    ) -> Vec<AppliedAuraEffectLikeCpp> {
        self.effects
            .iter()
            .filter(|e| e.aura_type == t)
            .copied()
            .collect()
    }
    fn player_amounts_of_type(
        &self,
        _: &HashMap<u8, AuraApplicationLikeCpp>,
        t: i32,
    ) -> Vec<(i32, i32)> {
        self.effects
            .iter()
            .filter(|e| e.aura_type == t)
            .map(|e| (e.misc_value, e.amount))
            .collect()
    }
    fn player_mechanic_mask(&self, _: &HashMap<u8, AuraApplicationLikeCpp>) -> u32 {
        0
    }
    fn creature_mechanic_mask(&self, _: &[AppliedAuraRef], _: u8) -> u32 {
        0
    }
    fn race_creature_type_mask(&self, _: u8) -> u32 {
        0
    }
    fn template_creature_type_mask(&self, _: u32) -> u32 {
        0
    }
    fn block_armor_constant(&self, _: u8) -> f32 {
        1.0
    }
    fn player_shields(
        &self,
        _: &HashMap<u8, AuraApplicationLikeCpp>,
        _: u8,
        _: u32,
    ) -> Vec<RepresentedAbsorbShieldLikeCpp> {
        Vec::new()
    }
    fn player_mana_shields(
        &self,
        _: &HashMap<u8, AuraApplicationLikeCpp>,
        _: u8,
        _: u32,
    ) -> Vec<RepresentedManaShieldLikeCpp> {
        Vec::new()
    }
    fn creature_shields(
        &self,
        _: &AuraSubsystem,
        _: u8,
        _: u32,
    ) -> Vec<RepresentedAbsorbShieldLikeCpp> {
        Vec::new()
    }
    fn share_player(
        &self,
        _: &HashMap<u8, AuraApplicationLikeCpp>,
    ) -> Vec<ShareAuraSnapshotLikeCpp> {
        self.shares.clone()
    }
    fn share_creature(&self, _: &[AppliedAuraRef], _: u8) -> Vec<ShareAuraSnapshotLikeCpp> {
        self.calls.borrow_mut().push("share");
        self.shares.clone()
    }
    fn threat_spell(&self, _: Option<i32>, _: u8) -> MeleeThreatSpellFacts {
        self.calls.borrow_mut().push("threat");
        MeleeThreatSpellFacts {
            suppress: false,
            no_initial_threat: false,
            school_mask: 1,
            multiplier: 1.0,
        }
    }
    fn threat_aura(&self, _: &[AppliedAuraRef], _: u8, _: u32) -> f32 {
        1.0
    }
    fn game_time_ms(&self) -> u64 {
        self.calls.borrow_mut().push("clock");
        123
    }
}

fn configure(actor: &mut crate::map_manager::WorldCreature, health: u64) {
    actor.creature.unit_mut().set_level(80);
    actor.creature.unit_mut().set_health(health);
    actor.creature.set_ai_identity_runtime(1, 35, 0, 0);
    actor
        .creature
        .set_avoidance_like_cpp(wow_entities::CreatureAvoidanceLikeCpp {
            dodge_pct: 0.0,
            parry_pct: 0.0,
            block_pct: 0.0,
        });
}
pub(super) fn target(
    manager: &mut MapManager,
    counter: i64,
    health: u64,
    record: bool,
) -> ObjectGuid {
    let mut actor = new_actor(counter, wow_core::Position::xyz(11.0, 20.0, 30.0), false);
    configure(&mut actor, health);
    let guid = actor.guid();
    let map = manager.find_map_mut(1, 0).unwrap().map_mut();
    if record {
        map.insert_map_object_record(MapObjectRecord::new_creature(actor.creature).unwrap())
            .unwrap();
    } else {
        assert!(matches!(
            map.admit_creature_actor(actor).unwrap(),
            CreatureActorAdmission::Inserted { .. }
        ));
    }
    place_in_loaded_cell(map, guid, wow_core::Position::xyz(11.0, 20.0, 30.0));
    guid
}
pub(super) fn setup(
    counter: i64,
    health: u64,
    record: bool,
    damage: u32,
) -> (MapManager, ObjectGuid, ObjectGuid) {
    let (mut manager, root) = manager_with_actor(counter);
    let victim = target(&mut manager, counter + 1, health, record);
    arm(&mut manager, root, victim, damage);
    (manager, root, victim)
}
pub(super) fn arm(manager: &mut MapManager, root: ObjectGuid, victim: ObjectGuid, damage: u32) {
    let actor = manager
        .find_map_mut(1, 0)
        .unwrap()
        .map_mut()
        .creature_actor_mut(root)
        .unwrap();
    configure(actor, 75);
    actor.enter_combat(victim);
    actor.creature.ai_ownership_mut().min_damage = damage;
    actor.creature.ai_ownership_mut().max_damage = damage;
    actor.creature.ai_ownership_mut().swing_timer_ms = 0;
    actor.seed_runtime_rng_like_cpp(17);
}
pub(super) fn health(manager: &MapManager, guid: ObjectGuid) -> u64 {
    manager
        .find_map(1, 0)
        .unwrap()
        .map()
        .with_creature_like_cpp(guid, |c| c.unit().data().health)
        .unwrap()
}
pub(super) fn captured(occurrence: &MeleeKillOccurrence) -> &CapturedMeleeKill {
    match occurrence.capture() {
        MeleeKillCapture::Captured(capture) => capture,
        MeleeKillCapture::Unavailable(error) => panic!("expected real Actor capture: {error:?}"),
    }
}
