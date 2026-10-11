// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The canonical creature's packet projection, runtime installation and the
//! creature operations the session handlers apply to it.
//!
//! #1263 F6-8E-2: the legacy `WorldCreature` bridge (canonical `Creature`
//! plus a cached `CreatureCreateData`) is deleted. The mutation root lends
//! `&mut Creature`; the create projection is computed on demand by
//! [`creature_create_data_like_cpp`]. The few bridge operations that were not
//! plain delegations to `Creature` are free functions here, with their bodies
//! unchanged.

use std::time::Instant;

use wow_constants::{PowerType, UnitDynFlags, UnitMoveType, WeaponAttackType};
use wow_core::{ObjectGuid, Position};
use wow_entities::creature_create::CreatureCreateData;
use wow_entities::{Creature, game_time_secs_like_cpp};
use wow_movement::WaypointPath;

use crate::creature_movement::CreatureMovementLikeCpp;
use crate::creature_respawn::{
    instant_from_respawn_time_like_cpp, restore_respawn_aura_source_authority_like_cpp,
};

/// C++ `BASE_ATTACK_TIME` (`UnitDefines.h:30`). Creature base/ranged attack time
/// is clamped to this when the template value is 0 (`ObjectMgr.cpp:1100-1104`); a
/// 0 attack time crashes the 3.4.3 client's swing-timer math on the first tick.
pub(crate) const BASE_ATTACK_TIME_LIKE_CPP: u32 = 2_000;

pub(crate) const fn power_type_from_u8_like_cpp(power: u8) -> PowerType {
    match power {
        1 => PowerType::Rage,
        2 => PowerType::Focus,
        3 => PowerType::Energy,
        4 => PowerType::Happiness,
        5 => PowerType::Runes,
        6 => PowerType::RunicPower,
        7 => PowerType::SoulShards,
        8 => PowerType::LunarPower,
        9 => PowerType::HolyPower,
        10 => PowerType::AlternatePower,
        11 => PowerType::Maelstrom,
        12 => PowerType::Chi,
        13 => PowerType::Insanity,
        14 => PowerType::ComboPoints,
        15 => PowerType::DemonicFury,
        16 => PowerType::ArcaneCharges,
        17 => PowerType::Fury,
        18 => PowerType::Pain,
        19 => PowerType::Essence,
        20 => PowerType::RuneBlood,
        21 => PowerType::RuneFrost,
        22 => PowerType::RuneUnholy,
        23 => PowerType::AlternateQuest,
        24 => PowerType::AlternateEncounter,
        25 => PowerType::AlternateMount,
        _ => PowerType::Mana,
    }
}

pub fn new_runtime_creature_like_cpp(
    guid: ObjectGuid,
    entry: u32,
    pos: Position,
    hp: u32,
    level: u8,
    min_dmg: u32,
    max_dmg: u32,
    aggro_radius: f32,
    display_id: u32,
    faction: u32,
    npc_flags: u32,
    unit_flags: u32,
) -> Creature {
    let (min_dmg, max_dmg) = if min_dmg == 0 {
        let base = (level as u32) * 3 + 5;
        (base, base + base / 2)
    } else {
        (min_dmg, max_dmg)
    };

    let mut creature = Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_entry(entry);
    creature.set_ai_position(pos);
    creature.set_ai_home_position(pos);
    creature.unit_mut().set_level(level);
    creature.unit_mut().set_max_health(u64::from(hp));
    creature.unit_mut().set_health(u64::from(hp));
    creature.set_ai_identity_runtime(display_id, faction, npc_flags, unit_flags);
    creature.unit_mut().set_weapon_damage(
        WeaponAttackType::BaseAttack,
        min_dmg as f32,
        max_dmg as f32,
    );
    {
        let ai = creature.ai_ownership_mut();
        ai.aggro_radius = aggro_radius;
        // C++ `Creature::Creature` initializes `m_wanderDistance` to 0.0f and only
        // random movement spawns get a positive distance from CreatureData.
        ai.wander_radius = 0.0;
        ai.respawn_time_secs = 30;
        ai.min_damage = min_dmg;
        ai.max_damage = max_dmg;
    }

    let create_data = CreatureCreateData {
        guid,
        entry,
        display_id,
        native_display_id: display_id,
        display_scale: 1.0,
        native_x_display_scale: 1.0,
        bounding_radius: 0.389,
        combat_reach: 1.5,
        health: hp as i64,
        max_health: hp as i64,
        level,
        faction_template: faction as i32,
        npc_flags: npc_flags as u64,
        unit_flags,
        unit_flags2: 0,
        unit_flags3: 0,
        aura_state: Creature::health_aura_state_like_cpp(hp as u64, hp as u64, hp > 0),
        damage_school: wow_constants::spell::SpellSchools::Normal as u8,
        scale: 1.0,
        unit_class: 1,
        display_power: 1,
        power: [0; 10],
        max_power: [0; 10],
        base_mana: 0,
        virtual_items: [(0, 0, 0); 3],
        base_attack_time: 2000,
        ranged_attack_time: 0,
        movement_flags: 0,
        vehicle_id: 0,
        play_hover_anim: false,
        hover_height: 1.0,
        mount_display_id: 0,
        stand_state: 0,
        vis_flags: 0,
        anim_tier: 0,
        emote_state: 0,
        sheathe_state: wow_constants::unit::SheathState::Melee as u8,
        pvp_flags: 0,
        current_area_id: 0,
        speed_walk_rate: 1.0,
        speed_run_rate: 1.14286,
        ai_anim_kit_id: 0,
        movement_anim_kit_id: 0,
        melee_anim_kit_id: 0,
    };

    install_creature_runtime_like_cpp(&mut creature, &create_data);
    creature
}

/// Install a fresh runtime incarnation on one canonical creature.
///
/// #1263 F6-8D3a-3: the respawn rebuild, the loaded-grid admission and the
/// session registration produce the same canonical incarnation through this
/// body. The runtime also keeps the create-time class/power projection the
/// respawn entry reads.
pub fn install_creature_runtime_like_cpp(
    creature: &mut Creature,
    create_data: &CreatureCreateData,
) {
    // This generic bridge carries no proof that every aura source was
    // hydrated. Preserve fail-closed semantics even if a caller passes a
    // clone that previously crossed a more authoritative boundary.
    creature
        .unit_mut()
        .subsystems_mut()
        .auras
        .invalidate_spell_hit_aura_authority_like_cpp();
    let _ = creature
        .unit_mut()
        .subsystems_mut()
        .motion
        .add_to_world_like_cpp();
    let runtime_motion_master = wow_entities::new_runtime_motion_master_like_cpp(creature);
    // #1263 F6-8A: this body installs a fresh runtime incarnation. The
    // persistent runtime state — including the elapsed-time clock, the RNG
    // and its authority marker — is reset exactly as the former
    // bridge-local field list reset it, and the canonical owner now stores
    // it. #1263 F6-8D1 added the motion-master advance counter to that same
    // reset.
    *creature.runtime_like_cpp_mut() =
        wow_entities::CreatureRuntimeLikeCpp::new_like_cpp(runtime_motion_master);
    creature.runtime_like_cpp_mut().set_respawn_create_projection_like_cpp(
        wow_entities::creature_create::CreatureRespawnCreateProjectionLikeCpp::from_create_data_like_cpp(
            create_data,
        ),
    );
}

/// Materialize the read-only visibility/create projection of one canonical
/// creature **carrying that creature's own canonical runtime state**.
///
/// #1263 F6-8B. [`install_creature_runtime_like_cpp`] installs a fresh runtime
/// incarnation, so it drops `Unit::movespline` and `Unit::i_motionMaster`,
/// and a create block built from such a projection loses the spline the
/// canonical owner holds. This projection therefore restores the creature's
/// canonical runtime state through
/// `CreatureRuntimeLikeCpp::cloned_for_like_cpp` (persistent spline,
/// generators and corridors cloned; `Unit::i_motionMaster` rebuilt for the
/// projection; chase target and represented-active key empty — the canonical
/// owner's own clone semantics).
///
/// A projection of the one canonical authority, **not a second mirror**:
/// nothing writes back through it and the canonical map stays the writer.
pub fn projected_creature_preserving_runtime_like_cpp(
    creature: Creature,
    create_data: &CreatureCreateData,
) -> Creature {
    let source = creature.clone();
    let mut projected = creature;
    install_creature_runtime_like_cpp(&mut projected, create_data);
    *projected.runtime_like_cpp_mut() = wow_entities::CreatureRuntimeLikeCpp::cloned_for_like_cpp(
        &projected,
        source.runtime_like_cpp(),
    );
    projected
}

/// The `CreatureCreateData` packet projection of one canonical creature,
/// computed on demand from its unit fields.
pub fn creature_create_data_like_cpp(creature: &Creature) -> CreatureCreateData {
    let unit = creature.unit();
    let data = unit.data();
    let object = unit.world().object();
    let npc_flags = unit.npc_flags_like_cpp();
    let attack_speed = unit.base_attack_speed();
    let speed_rate = unit.speed_rate();
    let vehicle_id = unit
        .subsystems()
        .vehicle
        .kit
        .as_ref()
        .map(|kit| kit.kit_id())
        .unwrap_or(0);

    CreatureCreateData {
        guid: creature.guid(),
        entry: creature.entry(),
        display_id: data.display_id.max(0) as u32,
        native_display_id: data.native_display_id.max(0) as u32,
        display_scale: data.display_scale,
        native_x_display_scale: data.native_display_scale,
        bounding_radius: data.bounding_radius,
        combat_reach: data.combat_reach,
        health: creature.current_health().min(i64::MAX as u64) as i64,
        max_health: creature.max_health().min(i64::MAX as u64) as i64,
        level: creature.level(),
        faction_template: data.faction_template,
        npc_flags: (u64::from(npc_flags[1]) << 32) | u64::from(npc_flags[0]),
        unit_flags: data.flags,
        unit_flags2: data.flags2,
        unit_flags3: data.flags3,
        aura_state: Creature::health_aura_state_like_cpp(
            creature.current_health(),
            creature.max_health(),
            creature.current_health() > 0,
        ),
        damage_school: creature.melee_damage_school_like_cpp(),
        scale: object.scale(),
        unit_class: data.class_id,
        display_power: data.display_power,
        power: data.power,
        max_power: data.max_power,
        base_mana: data.base_mana,
        virtual_items: [
            (
                data.virtual_items[0].item_id,
                data.virtual_items[0].item_appearance_mod_id,
                data.virtual_items[0].item_visual,
            ),
            (
                data.virtual_items[1].item_id,
                data.virtual_items[1].item_appearance_mod_id,
                data.virtual_items[1].item_visual,
            ),
            (
                data.virtual_items[2].item_id,
                data.virtual_items[2].item_appearance_mod_id,
                data.virtual_items[2].item_visual,
            ),
        ],
        // C++ guarantees UNIT_FIELD_BASEATTACKTIME is never 0: ObjectMgr.cpp:1100-1104
        // clamps creature_template BaseAttackTime/RangeAttackTime 0 -> BASE_ATTACK_TIME
        // (2000) at load. The 3.4.3 client divides by this on the first post-spawn unit
        // tick (swing-timer/attack-rate math), so a 0 here crashes the client a few
        // seconds after the create burst. Defense-in-depth clamp mirroring C++.
        base_attack_time: match attack_speed[WeaponAttackType::BaseAttack as usize] {
            0 => BASE_ATTACK_TIME_LIKE_CPP,
            t => t,
        },
        ranged_attack_time: match attack_speed[WeaponAttackType::RangedAttack as usize] {
            0 => BASE_ATTACK_TIME_LIKE_CPP,
            t => t,
        },
        movement_flags: creature.movement_flags_like_cpp().bits(),
        vehicle_id,
        play_hover_anim: false,
        hover_height: data.hover_height,
        mount_display_id: data.mount_display_id,
        stand_state: data.stand_state,
        vis_flags: data.vis_flags,
        anim_tier: data.anim_tier,
        emote_state: unit.emote_state_like_cpp() as i32,
        sheathe_state: data.sheathe_state,
        pvp_flags: data.pvp_flags,
        current_area_id: 0,
        speed_walk_rate: speed_rate[UnitMoveType::Walk as usize],
        speed_run_rate: speed_rate[UnitMoveType::Run as usize],
        ai_anim_kit_id: unit.ai_anim_kit_id_like_cpp(),
        movement_anim_kit_id: unit.movement_anim_kit_id_like_cpp(),
        melee_anim_kit_id: unit.melee_anim_kit_id_like_cpp(),
    }
}

/// Prepare one loaded-grid creature: a fresh runtime incarnation, the
/// DB-backed aura-source proofs and its default movement generator.
pub fn loaded_grid_creature_like_cpp(
    mut creature: Creature,
    mut waypoint_path_resolver: impl FnMut(u32) -> Option<WaypointPath>,
) -> Creature {
    let create_data = creature_create_data_like_cpp(&creature);
    install_creature_runtime_like_cpp(&mut creature, &create_data);
    // The loaded-grid lifecycle receives a Creature only after the
    // DB-backed creature_addon/template_addon store has resolved and the
    // selected addon has been applied to its canonical AuraSubsystem.
    restore_respawn_aura_source_authority_like_cpp(&mut creature, true, true);
    match creature.default_movement_type() {
        wow_entities::MovementGeneratorType::Random => {
            CreatureMovementLikeCpp::canonical_like_cpp(&mut creature)
                .initialize_default_random_movement_like_cpp();
        }
        wow_entities::MovementGeneratorType::Waypoint => {
            CreatureMovementLikeCpp::canonical_like_cpp(&mut creature)
                .initialize_default_waypoint_movement_with_path_resolver_like_cpp(|path_id| {
                    waypoint_path_resolver(path_id)
                });
        }
        wow_entities::MovementGeneratorType::Idle => {}
    }
    creature
}

/// Apply C++'s taunt Aura using the parent `Spell` object's retained cast
/// identity. `EffectTaunt` must not allocate a second Cast GUID for the Aura
/// base created by the same cast.
///
/// `SPELL_AURA_MOD_TAUNT` lives in the static-data layer the canonical entity
/// crate must not depend on, so this entry supplies it.
pub fn apply_taunt_aura_with_provenance_like_cpp(
    creature: &mut Creature,
    caster: ObjectGuid,
    spell_id: u32,
    effect_mask: u32,
    duration_ms: i32,
    provenance: wow_entities::AuraCastProvenanceLikeCpp,
) -> Option<u8> {
    creature.apply_taunt_aura_with_provenance_like_cpp(
        caster,
        spell_id,
        effect_mask,
        duration_ms,
        provenance,
        wow_data::spell::aura_types::SPELL_AURA_MOD_TAUNT,
    )
}

/// [`apply_taunt_aura_with_provenance_like_cpp`] with a default provenance.
pub fn apply_taunt_aura_like_cpp(
    creature: &mut Creature,
    caster: ObjectGuid,
    spell_id: u32,
    effect_mask: u32,
    duration_ms: i32,
) -> Option<u8> {
    apply_taunt_aura_with_provenance_like_cpp(
        creature,
        caster,
        spell_id,
        effect_mask,
        duration_ms,
        wow_entities::AuraCastProvenanceLikeCpp::default(),
    )
}

/// C++ `Creature::AllLootRemovedFromCorpse` at the current game time.
pub fn all_loot_removed_from_corpse_like_cpp(
    creature: &mut Creature,
    decay_rate: f32,
    is_fully_skinned: bool,
) -> bool {
    all_loot_removed_from_corpse_at_game_time_like_cpp(
        creature,
        Instant::now(),
        game_time_secs_like_cpp(),
        decay_rate,
        is_fully_skinned,
    )
}

/// C++ `Creature::AllLootRemovedFromCorpse`: the corpse removal time, kept on
/// the creature-local clock.
pub fn all_loot_removed_from_corpse_at_game_time_like_cpp(
    creature: &mut Creature,
    now: Instant,
    game_time_secs: i64,
    decay_rate: f32,
    is_fully_skinned: bool,
) -> bool {
    let plan = creature.all_loot_removed_from_corpse(game_time_secs, decay_rate, is_fully_skinned);
    if plan.is_empty() {
        return false;
    }

    // C++ stores `m_corpseRemoveTime` in the absolute GameTime domain.
    // Translate only the remaining duration into the creature's logical
    // Map-tick clock; wall time is not an ongoing runtime clock source.
    let corpse_remove_at =
        instant_from_respawn_time_like_cpp(creature.corpse_remove_time(), now, game_time_secs);
    let remaining_ms = corpse_remove_at
        .checked_duration_since(now)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0);
    let corpse_remove_time_ms = creature
        .runtime_elapsed_ms_like_cpp()
        .saturating_add(remaining_ms);
    creature.set_ai_corpse_despawn_at(Some(corpse_remove_time_ms));
    true
}

/// `RemoveDynamicFlag(UNIT_DYNFLAG_LOOTABLE)` plus the forced VALUES update.
pub fn remove_lootable_dynamic_flag_like_cpp(creature: &mut Creature) {
    let object = creature.unit_mut().world_mut().object_mut();
    object.remove_dynamic_flag(UnitDynFlags::Lootable as u32);
    object.force_dynamic_flags_update_like_cpp();
}

/// Force the next VALUES update to carry `UNIT_DYNAMIC_FLAGS`.
pub fn force_dynamic_flags_update_like_cpp(creature: &mut Creature) {
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .force_dynamic_flags_update_like_cpp();
}

/// Whether `UNIT_DYNFLAG_LOOTABLE` is set.
pub fn has_lootable_dynamic_flag_like_cpp(creature: &Creature) -> bool {
    creature
        .unit()
        .world()
        .object()
        .has_dynamic_flag(UnitDynFlags::Lootable as u32)
}

/// Test accessors of the canonical creature (the former `WorldCreature`
/// fixture methods).
#[cfg(any(test, feature = "test-fixtures"))]
pub mod test_support {
    use std::time::{Duration, Instant};

    use wow_entities::{Creature, game_time_secs_like_cpp};

    use crate::creature_respawn::creature_respawn_at_from_death_at_game_time_like_cpp;

    /// Seed the creature-owned runtime RNG.
    pub fn seed_runtime_rng_like_cpp(creature: &mut Creature, seed: u64) {
        creature
            .runtime_like_cpp_mut()
            .seed_runtime_rng_like_cpp(seed);
    }

    /// Set the creature-local clock to `elapsed`.
    pub fn backdate_runtime_clock_for_test(creature: &mut Creature, elapsed: Duration) {
        creature
            .runtime_like_cpp_mut()
            .set_runtime_elapsed_ms_like_cpp(elapsed.as_millis().min(u128::from(u64::MAX)) as u64);
    }

    /// Milliseconds until one creature spell slot is due, if scheduled.
    pub fn creature_spell_due_in_ms_for_test(creature: &Creature, slot: usize) -> Option<u64> {
        creature
            .runtime_like_cpp()
            .creature_spell_due_at_ms_like_cpp
            .get(slot)
            .copied()
            .flatten()
            .map(|due_at| due_at.saturating_sub(creature.runtime_elapsed_ms_like_cpp()))
    }

    /// The corpse despawn deadline on the creature-local clock.
    pub const fn corpse_despawn_deadline_ms_like_cpp(creature: &Creature) -> Option<u64> {
        creature.ai_ownership().corpse_despawn_at_ms
    }

    /// The corpse despawn deadline as a wall-clock instant.
    pub fn corpse_despawn_at(creature: &Creature) -> Option<Instant> {
        let now = Instant::now();
        let elapsed_ms = creature.runtime_elapsed_ms_like_cpp();
        creature
            .ai_ownership()
            .corpse_despawn_at_ms
            .map(|due_at_ms| now + Duration::from_millis(due_at_ms.saturating_sub(elapsed_ms)))
    }

    /// Set the corpse despawn deadline from a wall-clock instant.
    pub fn set_corpse_despawn_at(creature: &mut Creature, when: Option<Instant>) {
        let now = Instant::now();
        let now_ms = creature.runtime_elapsed_ms_like_cpp();
        let at_ms = when.map(|instant| {
            if instant <= now {
                now_ms
            } else {
                now_ms.saturating_add(
                    instant
                        .duration_since(now)
                        .as_millis()
                        .min(u128::from(u64::MAX)) as u64,
                )
            }
        });
        creature.set_ai_corpse_despawn_at(at_ms);
    }

    /// The respawn deadline at the current game time.
    pub fn respawn_at_from_death_like_cpp(creature: &Creature) -> Instant {
        creature_respawn_at_from_death_at_game_time_like_cpp(
            creature,
            Instant::now(),
            game_time_secs_like_cpp(),
        )
    }

    /// Whether the dead creature's respawn is due on its local clock.
    pub fn should_respawn(creature: &Creature) -> bool {
        creature.should_ai_respawn(creature.runtime_elapsed_ms_like_cpp())
    }
}
