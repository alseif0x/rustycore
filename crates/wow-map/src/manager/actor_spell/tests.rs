//! Whole actors in real maps; the existing lifetime fixture constructs motors by move.
use super::*;
use crate::manager::{MapObjectUpdateSelectionLikeCpp, ObjectMapTickError};
use crate::manager::actor_tick_access::fixtures;
use crate::map_manager::{WorldCreature, AggroAiFacts, SpellAiKind, SpellCondition,
    SpellTarget, SpellDisable, SpellInfoFacts, SpellEffectFacts, SpellPowerFacts,
    SpellPreparationCheck, SpellCooldown, SpellRange, SpellHitFacts, SpellFactionFacts,
    SpellHit, SpellHitProfile, represented_hit_profile, resolve_hit_profile, cast_log};
use wow_core::Position;
use wow_entities::{MapObjectRecord, Player};
use std::cell::RefCell;
use std::rc::Rc;

// Evaluate preparation/resume before borrowing the manager for the driver.
macro_rules! complete {
    ($manager:expr, $tick:expr, $token:expr, $settings:expr, $trace:expr, $initial:expr) => {{
        let progress = $initial;
        drive($manager, $tick, $token, $settings, $trace, progress)
    }};
}

mod catalog;
mod order;
mod rejection;
mod equivalence;
mod publication;

type Trace = Rc<RefCell<Vec<&'static str>>>;
#[derive(Clone, Copy)]
struct Settings {
    ai: SpellAiKind, condition: SpellCondition, mixed: bool, target: SpellTarget,
    disable: SpellDisable, missing: bool, visual_missing: bool, ignore_los: bool,
    hit_missing: bool, no_miss: bool, range: SpellRange, cooldown: Option<SpellCooldown>,
}
impl Default for Settings {
    fn default() -> Self {
        Self { ai: SpellAiKind::Combat, condition: SpellCondition::Aggro, mixed: false,
            target: SpellTarget::Victim, disable: SpellDisable::Enabled, missing: false,
            visual_missing: false, ignore_los: false, hit_missing: false, no_miss: false,
            range: SpellRange { minimum: 0.0, maximum: 30.0, flags: 0 }, cooldown: None }
    }
}

fn info(id: u32) -> SpellInfoFacts {
    SpellInfoFacts { spell_id: id as i32, cast_time_ms: 0, requires_spell_focus: 0,
        effect_type: 2, aura_type: None, effect_base_points: 10,
        effects: vec![SpellEffectFacts { effect: 2, effect_index: 0, effect_aura: 0,
            effect_base_points: 10, implicit_target_1: 6, implicit_target_2: 0,
            chain_targets: 0, effect_radius_index_1: 0, effect_trigger_spell: 0 }],
        power_costs: Vec::new() }
}

fn policy<R>(settings: Settings, trace: &Trace, apply: impl FnOnce(&mut SpellPolicies<'_>) -> R) -> R {
    let mut ai = |_: AggroAiFacts<'_>| { trace.borrow_mut().push("ai"); settings.ai };
    let mut info = |id, _, effective| {
        trace.borrow_mut().push(if effective { "effective" } else { "base" });
        if settings.missing { None } else { Some(info(id)) }
    };
    let mut condition = |id, _| {
        trace.borrow_mut().push("condition");
        if settings.mixed && id == 70_102 { SpellCondition::Combat } else { settings.condition }
    };
    let mut target = |_, _: &SpellInfoFacts, _| { trace.borrow_mut().push("target"); settings.target };
    let mut disable = |_, _, _| { trace.borrow_mut().push("disable"); settings.disable };
    let mut check = |check, _, _| {
        trace.borrow_mut().push(match check {
            SpellPreparationCheck::RuntimeHooks => "hooks", SpellPreparationCheck::CastingRequirements => "requirements",
            SpellPreparationCheck::ShapeshiftRequirements => "shapeshift", SpellPreparationCheck::AuraRestrictions => "auras",
            SpellPreparationCheck::CooldownSemantics => "cooldown_semantics", SpellPreparationCheck::CombatForbidden => "combat_forbidden",
            SpellPreparationCheck::TargetRestrictions => "target_restrictions", SpellPreparationCheck::Projectile => "projectile",
        });
        matches!(check, SpellPreparationCheck::RuntimeHooks | SpellPreparationCheck::CastingRequirements
            | SpellPreparationCheck::ShapeshiftRequirements | SpellPreparationCheck::AuraRestrictions | SpellPreparationCheck::CooldownSemantics)
    };
    let mut attributes = |_, _| {
        trace.borrow_mut().push("attributes");
        let mut attributes = [0; 15]; attributes[0] = 0x10;
        if settings.ignore_los { attributes[2] |= 4; }
        if settings.no_miss { attributes[7] |= 0x0200_0000; }
        Some(attributes)
    };
    let mut has_attribute = |_, _, _, _| { trace.borrow_mut().push("reset_attribute"); false };
    let mut minimum = |_, _| { trace.borrow_mut().push("minimum"); 6_000 };
    let mut visual = |_, _| { trace.borrow_mut().push("visual"); if settings.visual_missing { Err(()) } else { Ok(23) } };
    let mut flags = |_, _| { trace.borrow_mut().push("go_flags"); 0x100 };
    let mut cooldown = |_, _| { trace.borrow_mut().push("cooldown"); settings.cooldown };
    let mut range = |_, _| { trace.borrow_mut().push("range"); Some(settings.range) };
    let mut hit_metadata = |_, _| {
        trace.borrow_mut().push("hit_metadata");
        if settings.hit_missing { None } else {
            Some(SpellHitFacts { defense_type: 2, school_mask: 1, spell_mechanic: 0,
                effect_mechanics: [(0, 0)].into_iter().collect() })
        }
    };
    let mut faction_authority = || true;
    let mut factions = |_, _| { trace.borrow_mut().push("factions"); Some(SpellFactionFacts {
        creature_faction_id: 0, contested_guard: false, caster_hostile: true, victim_hostile: true,
        caster_friendly: false, victim_friendly: false }) };
    let mut reputation = |_| Some(false);
    apply(&mut SpellPolicies { select_ai: &mut ai, info: &mut info, condition: &mut condition,
        target: &mut target, disable: &mut disable, check: &mut check, attributes: &mut attributes,
        has_attribute: &mut has_attribute, minimum: &mut minimum, visual: &mut visual, go_flags: &mut flags,
        cooldown: &mut cooldown, range: &mut range, hit_metadata: &mut hit_metadata,
        faction_authority: &mut faction_authority, factions: &mut factions, can_have_reputation: &mut reputation })
}

fn actor(manager: &MapManager, guid: ObjectGuid) -> &WorldCreature {
    manager.find_map(1, 0).unwrap().map().creature_actor(guid).unwrap()
}
fn actor_mut(manager: &mut MapManager, guid: ObjectGuid) -> &mut WorldCreature {
    manager.find_map_mut(1, 0).unwrap().map_mut().creature_actor_mut(guid).unwrap()
}
fn prefix(manager: &MapManager, guid: ObjectGuid) -> (u64, u64, u32) {
    let actor = actor(manager, guid);
    (actor.runtime_elapsed_ms_like_cpp(), actor.runtime_motion_master_ticks_like_cpp(), actor.spline_id())
}
fn add_player(manager: &mut MapManager, counter: i64) -> ObjectGuid {
    let guid = ObjectGuid::create_player(1, counter);
    let mut player = Player::new(None, false);
    player.unit_mut().world_mut().object_mut().create(guid);
    player.unit_mut().world_mut().set_map(1, 0).unwrap();
    player.unit_mut().world_mut().relocate(Position::xyz(11.0, 20.0, 30.0));
    player.unit_mut().world_mut().object_mut().add_to_world();
    player.unit_mut().set_max_health(100); player.unit_mut().set_health(100);
    player.unit_mut().set_faction(1);
    player.unit_mut().subsystems_mut().auras.set_spell_hit_aura_authority_inert_like_cpp(true);
    manager.find_map_mut(1, 0).unwrap().map_mut().insert_map_object_record(MapObjectRecord::new_player(player).unwrap()).unwrap();
    manager.adopt_active_player_like_cpp(guid).unwrap();
    guid
}
fn configure(incoming: &mut WorldCreature, victim: ObjectGuid) {
    incoming.creature.set_faction(14);
    incoming.creature.set_spell(0, 70_101);
    incoming.creature.unit_mut().subsystems_mut().auras.set_spell_hit_aura_authority_inert_like_cpp(true);
    incoming.creature.unit_mut().subsystems_mut().auras.set_spell_cast_log_aura_authority_inert_like_cpp(true);
    incoming.seed_runtime_rng_like_cpp(777);
    incoming.enter_combat(victim);
    incoming.creature.ai_ownership_mut().last_swing_ms = 0;
    incoming.creature.ai_ownership_mut().swing_timer_ms = 0;
}
fn setup(counter: i64) -> (MapManager, ObjectGuid, ObjectGuid) {
    let (mut manager, guid) = fixtures::manager_with_actor(counter);
    let victim = add_player(&mut manager, counter + 1);
    configure(actor_mut(&mut manager, guid), victim);
    (manager, guid, victim)
}
fn drive(manager: &mut MapManager, tick: &MapObjectTickContinuation,
    token: &mut ObjectMapUpdateToken, settings: Settings, trace: &Trace,
    mut progress: ActorSpellProgress) -> SpellOutcome {
    let mut completions = Vec::new();
    loop {
        progress = match progress {
            ActorSpellProgress::Complete(mut outcome) => {
                outcome.completions.extend(completions);
                return outcome;
            }
            ActorSpellProgress::Publication { completion, continuation } => {
                completions.push(completion);
                policy(settings, trace, |policies|
                    manager.resume_spell_publication(tick, token, continuation, policies)).unwrap()
            }
            ActorSpellProgress::Pending(_) => panic!("unexpected query"),
        };
    }
}
fn pending(counter: i64) -> (MapManager, MapObjectTickContinuation, ObjectMapUpdateToken, ObjectGuid, ObjectGuid, ActorSpellLosRequest) {
    let (mut manager, guid, victim) = setup(counter);
    let (tick, mut token) = fixtures::start(&mut manager, 77, MapObjectUpdateSelectionLikeCpp::WholeTypedStores);
    let progress = policy(Settings::default(), &Trace::default(), |policies| manager.prepare_spell(&tick, &mut token, true, policies)).unwrap();
    let ActorSpellProgress::Pending(request) = progress else { panic!("LOS pending") };
    (manager, tick, token, guid, victim, request)
}
