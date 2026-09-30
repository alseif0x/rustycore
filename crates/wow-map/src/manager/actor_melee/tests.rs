//! Real token admissions and borrowed canonical melee; no production activation.
use super::*;
use crate::SpawnObjectType;
use crate::manager::actor_tick_access::fixtures::*;
use crate::manager::{
    MapCreatureUpdateOwnerLikeCpp, MapObjectUpdateSelectionLikeCpp,
    MapTickCoordinationStateLikeCpp, ObjectMapFinishOutcome,
};
use crate::map::{CreatureActorAdmission, Map, MeleeThreatSpellFacts, ShareAuraSnapshotLikeCpp};
use crate::spawn::SpawnId;
use std::cell::RefCell;
use std::collections::HashMap;
use wow_combat::{
    AppliedAuraEffectLikeCpp, RepresentedAbsorbShieldLikeCpp, RepresentedManaShieldLikeCpp,
};
use wow_core::Position;
use wow_entities::{
    AppliedAuraRef, AuraApplicationLikeCpp, AuraSubsystem, MapObjectRecord, Player,
};

mod gates;
mod selection;

#[derive(Default)]
struct Catalogs {
    calls: RefCell<Vec<&'static str>>,
}
impl Catalogs {
    fn empty<T>(&self, label: &'static str) -> Vec<T> {
        self.calls.borrow_mut().push(label);
        Vec::new()
    }
}
impl CreatureMeleeCatalogsLikeCpp for Catalogs {
    fn represented(&self) -> bool {
        self.calls.borrow_mut().push("represented");
        false
    }
    fn creature_effects(&self, _: &[AppliedAuraRef], _: u8) -> Vec<AppliedAuraEffectLikeCpp> {
        self.empty("creature_effects")
    }
    fn player_effects(
        &self,
        _: &HashMap<u8, AuraApplicationLikeCpp>,
    ) -> Vec<AppliedAuraEffectLikeCpp> {
        self.empty("player_effects")
    }
    fn player_effects_of_type(
        &self,
        _: &HashMap<u8, AuraApplicationLikeCpp>,
        _: i32,
    ) -> Vec<AppliedAuraEffectLikeCpp> {
        self.empty("player_type")
    }
    fn player_amounts_of_type(
        &self,
        _: &HashMap<u8, AuraApplicationLikeCpp>,
        _: i32,
    ) -> Vec<(i32, i32)> {
        self.empty("player_amounts")
    }
    fn player_mechanic_mask(&self, _: &HashMap<u8, AuraApplicationLikeCpp>) -> u32 {
        self.calls.borrow_mut().push("player_mechanic");
        0
    }
    fn creature_mechanic_mask(&self, _: &[AppliedAuraRef], _: u8) -> u32 {
        self.calls.borrow_mut().push("creature_mechanic");
        0
    }
    fn race_creature_type_mask(&self, _: u8) -> u32 {
        self.calls.borrow_mut().push("race");
        0
    }
    fn template_creature_type_mask(&self, _: u32) -> u32 {
        self.calls.borrow_mut().push("template");
        0
    }
    fn block_armor_constant(&self, _: u8) -> f32 {
        self.calls.borrow_mut().push("armor");
        1.0
    }
    fn player_shields(
        &self,
        _: &HashMap<u8, AuraApplicationLikeCpp>,
        _: u8,
        _: u32,
    ) -> Vec<RepresentedAbsorbShieldLikeCpp> {
        self.empty("player_shields")
    }
    fn player_mana_shields(
        &self,
        _: &HashMap<u8, AuraApplicationLikeCpp>,
        _: u8,
        _: u32,
    ) -> Vec<RepresentedManaShieldLikeCpp> {
        self.empty("mana")
    }
    fn creature_shields(
        &self,
        _: &AuraSubsystem,
        _: u8,
        _: u32,
    ) -> Vec<RepresentedAbsorbShieldLikeCpp> {
        self.empty("creature_shields")
    }
    fn share_player(
        &self,
        _: &HashMap<u8, AuraApplicationLikeCpp>,
    ) -> Vec<ShareAuraSnapshotLikeCpp> {
        self.empty("player_share")
    }
    fn share_creature(&self, _: &[AppliedAuraRef], _: u8) -> Vec<ShareAuraSnapshotLikeCpp> {
        self.empty("creature_share")
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
        self.calls.borrow_mut().push("threat_aura");
        1.0
    }
    fn game_time_ms(&self) -> u64 {
        self.calls.borrow_mut().push("clock");
        123
    }
}

type LoadRecord =
    fn(&mut Map, SpawnObjectType, SpawnId) -> Option<crate::map::LoadedGridRespawnRecordsLikeCpp>;

fn finish(
    manager: &mut MapManager,
    tick: &mut MapObjectTickContinuation,
    token: ObjectMapUpdateToken,
) -> ObjectMapFinishOutcome {
    match manager.try_finish_object_map::<LoadRecord>(
        tick,
        token,
        None,
        None,
        MapCreatureUpdateOwnerLikeCpp::ExternalRuntime,
    ) {
        Ok(outcome) => outcome,
        Err((error, _token)) => panic!("melee fixture must retain finish ownership: {error:?}"),
    }
}

fn setup(counter: i64) -> (MapManager, ObjectGuid, ObjectGuid) {
    let (mut manager, attacker) = manager_with_actor(counter);
    let victim = ObjectGuid::create_player(1, counter + 1);
    let mut player = Player::new(Some(1), false);
    player.unit_mut().world_mut().object_mut().create(victim);
    player.unit_mut().world_mut().set_map(1, 0).unwrap();
    player
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(11.0, 20.0, 30.0));
    player.unit_mut().world_mut().object_mut().add_to_world();
    player.unit_mut().set_level(80);
    player.unit_mut().set_max_health(100);
    player.unit_mut().set_health(100);
    let map = manager.find_map_mut(1, 0).unwrap().map_mut();
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    let actor = map.creature_actor_mut(attacker).unwrap();
    actor.enter_combat(victim);
    actor.creature.ai_ownership_mut().min_damage = 10;
    actor.creature.ai_ownership_mut().max_damage = 10;
    actor.creature.ai_ownership_mut().swing_timer_ms = 0;
    actor.seed_runtime_rng_like_cpp(17);
    (manager, attacker, victim)
}

fn assert_untouched(manager: &MapManager, attacker: ObjectGuid, victim: ObjectGuid) {
    let map = manager.find_map(1, 0).unwrap();
    assert_eq!(
        map.map()
            .get_typed_player(victim)
            .unwrap()
            .unit()
            .data()
            .health,
        100
    );
    let actor = map.map().creature_actor(attacker).unwrap();
    assert_eq!(actor.creature.ai_ownership().swing_timer_ms, 0);
    assert!(actor.runtime_rng_authority_complete_like_cpp());
    assert_eq!(map.last_creatures_update_summary().visited, 0);
    assert!(
        !map.last_map_update_tail_summary_like_cpp()
            .script_hook
            .invoked
    );
    assert_eq!(manager.updater.pending_requests, 1);
}
