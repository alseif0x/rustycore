// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1263 F6-8D3a-1: where the player melee phase finds a creature victim.
//!
//! The player melee swing body is written once
//! ([`execute_player_melee_swing_on_manager_like_cpp`]). The creature victim is
//! the canonical incarnation on the map the executor holds locked (#1263
//! F6-8E: the only creature store); the swing reads it through
//! [`player_melee_creature_victim_facts_like_cpp`] and applies the swing
//! through `apply_player_melee_to_creature_like_cpp`.

use super::*;

/// What one creature-victim read resolved.
pub(in crate::session) enum CreatureVictimReadLikeCpp<R> {
    /// The victim is not a creature of this map; the caller tries a player.
    NotACreature,
    Read(R),
}

/// What one creature-victim swing application resolved.
pub(in crate::session) enum CreatureVictimApplyLikeCpp {
    Missing,
    NotApplied,
    Hit(PlayerMeleeCreatureHitLikeCpp),
}

/// The admitted canonical executor's creature victims: the canonical
/// incarnation itself, on the map the executor holds locked. The swing writes
/// it once; there is no representation to mirror.
pub(in crate::session) struct CanonicalPlayerMeleeCreatureVictimsLikeCpp;

impl CanonicalPlayerMeleeCreatureVictimsLikeCpp {
    pub(in crate::session) fn read_creature_victim_like_cpp<R>(
        &mut self,
        map: &wow_map::ManagedMapInnerLikeCpp,
        _map_id: u16,
        _instance_id: u32,
        victim_guid: ObjectGuid,
        read: impl FnOnce(&wow_entities::Creature) -> R,
    ) -> CreatureVictimReadLikeCpp<R> {
        match map.get_typed_creature(victim_guid) {
            Some(creature) => CreatureVictimReadLikeCpp::Read(read(creature)),
            None => CreatureVictimReadLikeCpp::NotACreature,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::session) fn apply_to_creature_victim_like_cpp(
        &mut self,
        map: &mut wow_map::ManagedMapInnerLikeCpp,
        _map_id: u16,
        _instance_id: u32,
        victim_guid: ObjectGuid,
        player_guid: ObjectGuid,
        tap_group_guids: &[ObjectGuid],
        damages: &[crate::session::combat::RepresentedMeleeSwingLikeCpp],
    ) -> CreatureVictimApplyLikeCpp {
        match map.with_creature_mut_like_cpp(victim_guid, |creature| {
            apply_player_melee_to_creature_like_cpp(
                creature,
                player_guid,
                tap_group_guids,
                Some(damages),
            )
        }) {
            None => CreatureVictimApplyLikeCpp::Missing,
            Some(None) => CreatureVictimApplyLikeCpp::NotApplied,
            Some(Some(hit)) => CreatureVictimApplyLikeCpp::Hit(hit),
        }
    }

    /// The swing no longer needs any creature victim (a player victim).
    pub(in crate::session) fn release_creature_victims_like_cpp(&mut self) {}

    /// Whether the victim's own map publishes the VALUES the swing changed.
    ///
    /// #1263 F6-8D3b-3: C++ `Unit::AttackerStateUpdate` → `DealDamage` →
    /// `SetHealth` only marks the field; `Map::SendObjectUpdates` publishes it
    /// once at the end of the map update (`Map.cpp:1929-1948`). The canonical
    /// victim is exactly that map record, so the melee result must not carry a
    /// second copy to the attacker's client.
    pub(in crate::session) fn victim_values_published_by_map_like_cpp(&self) -> bool {
        true
    }
}

/// The creature-victim terms one player swing reads before it is resolved.
pub(in crate::session) struct PlayerMeleeCreatureVictimFactsLikeCpp {
    pub(in crate::session) aura_state_mask: u32,
    pub(in crate::session) mechanic_mask: u64,
    pub(in crate::session) armor: i32,
    pub(in crate::session) level: u8,
    pub(in crate::session) applied_auras: Vec<wow_entities::AppliedAuraRef>,
    pub(in crate::session) outcome_facts: crate::session_rules::RepresentedMeleeVictimFactsLikeCpp,
    pub(in crate::session) creature_type_mask: u32,
    pub(in crate::session) position: Position,
    pub(in crate::session) combat_reach: f32,
    pub(in crate::session) bounding_radius: f32,
}

/// Read one creature victim for a player swing. `None` for a dead victim.
pub(in crate::session) fn player_melee_creature_victim_facts_like_cpp(
    creature: &wow_entities::Creature,
    config: &LegacyCreatureAggroConfigLikeCpp,
    map_difficulty_id: u8,
    attacker_guid: ObjectGuid,
) -> Option<PlayerMeleeCreatureVictimFactsLikeCpp> {
    if !creature.is_alive() {
        return None;
    }
    // C++ `Unit::GetCreatureTypeMask` for the victim template.
    let victim_aura_state_mask = {
        let unit = creature.unit();
        unit.subsystems().auras.aura_state_mask
            | wow_entities::Creature::health_aura_state_like_cpp(
                creature.current_hp() as u64,
                creature.max_hp() as u64,
                creature.is_alive(),
            )
    };
    // C++ `Unit::HasAuraWithMechanic` for the victim template, from the
    // same receiver-free rule the session target path uses.
    let victim_mechanic_mask = config.spell_store.as_deref().map_or(0, |spell_store| {
        crate::session_rules::applied_aura_mechanic_mask_like_cpp(
            &creature.unit().subsystems().auras.applied_auras,
            spell_store,
            map_difficulty_id,
            config.difficulty_store.as_deref(),
        )
    });
    let victim_armor = creature.combat_log_stats_like_cpp().armor;
    let victim_level = creature.level();
    let creature_applied_auras = creature.unit().subsystems().auras.applied_auras.clone();
    // The victim's avoidance and attacker-facing aura terms
    // (`Unit::GetUnitDodgeChance` and friends, `Unit.cpp:2313-2378`).
    let victim_aura_sum = |aura_type: i32| -> f32 {
        config.spell_store.as_deref().map_or(0.0, |spell_store| {
            crate::session_rules::creature_aura_effects_like_cpp(
                &creature_applied_auras,
                spell_store,
                map_difficulty_id,
                config.difficulty_store.as_deref(),
            )
            .into_iter()
            .filter(|effect| effect.aura_type == aura_type)
            .map(|effect| effect.amount as f32)
            .sum()
        })
    };
    let victim_outcome_facts = crate::session_rules::RepresentedMeleeVictimFactsLikeCpp {
        level: victim_level,
        is_creature: true,
        is_player: false,
        is_stand_state: true,
        is_immune_to_damage: false,
        is_totem: creature.is_totem_unit_type_like_cpp(),
        is_evading_attacks: creature.is_evading_attacks_like_cpp(),
        dodge_pct: creature.avoidance_like_cpp().dodge_pct,
        parry_pct: creature.avoidance_like_cpp().parry_pct,
        block_pct: creature.avoidance_like_cpp().block_pct,
        dodge_aura_pct: victim_aura_sum(wow_data::spell::aura_types::SPELL_AURA_MOD_DODGE_PERCENT),
        parry_aura_pct: victim_aura_sum(wow_data::spell::aura_types::SPELL_AURA_MOD_PARRY_PERCENT),
        block_aura_pct: victim_aura_sum(wow_data::spell::aura_types::SPELL_AURA_MOD_BLOCK_PERCENT),
        attacker_melee_hit_chance_pct: victim_aura_sum(
            wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE,
        ),
        attacker_melee_crit_chance_pct: victim_aura_sum(
            wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACKER_MELEE_CRIT_CHANCE,
        ) + victim_aura_sum(
            wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACKER_SPELL_AND_WEAPON_CRIT_CHANCE,
        ),
        // C++ `GetUnitCriticalChanceTaken`'s conditional terms:
        // `!HealthBelowPct(MiscValueB)` and `caster == attacker`.
        crit_chance_vs_target_health_pct: config.spell_store.as_deref().map_or(
            0.0,
            |spell_store| {
                let health_pct = if creature.max_hp() == 0 {
                    100.0
                } else {
                    100.0 * creature.current_hp() as f32 / creature.max_hp() as f32
                };
                crate::session_rules::creature_aura_effects_like_cpp(
                    &creature_applied_auras,
                    spell_store,
                    map_difficulty_id,
                    config.difficulty_store.as_deref(),
                )
                .into_iter()
                .filter(|effect| {
                    effect.aura_type
                        == wow_data::spell::aura_types::
                            SPELL_AURA_MOD_CRIT_CHANCE_VERSUS_TARGET_HEALTH
                        && health_pct >= effect.misc_value_b as f32
                })
                .map(|effect| effect.amount as f32)
                .sum()
            },
        ),
        crit_chance_for_caster_pct: config.spell_store.as_deref().map_or(0.0, |spell_store| {
            crate::session_rules::creature_aura_effects_like_cpp(
                &creature_applied_auras,
                spell_store,
                map_difficulty_id,
                config.difficulty_store.as_deref(),
            )
            .into_iter()
            .filter(|effect| {
                effect.aura_type
                    == wow_data::spell::aura_types::SPELL_AURA_MOD_CRIT_CHANCE_FOR_CASTER
                    && effect.caster_guid == attacker_guid
            })
            .map(|effect| effect.amount as f32)
            .sum()
        }),
        faces_attacker: false,
        is_controlled: creature
            .unit()
            .has_unit_state(wow_constants::unit::UnitState::CONTROLLED.bits()),
    };
    let victim_creature_type_mask = config
        .creature_template_lifecycle_store
        .as_ref()
        .and_then(|store| store.get(creature.entry()))
        .map(|template| {
            if template.creature_type >= 1 {
                1_u32 << (template.creature_type - 1)
            } else {
                0
            }
        })
        .unwrap_or(0);
    let unit_data = creature.unit().data();
    Some(PlayerMeleeCreatureVictimFactsLikeCpp {
        aura_state_mask: victim_aura_state_mask,
        mechanic_mask: victim_mechanic_mask,
        armor: victim_armor,
        level: victim_level,
        applied_auras: creature_applied_auras,
        outcome_facts: victim_outcome_facts,
        creature_type_mask: victim_creature_type_mask,
        position: creature.position(),
        combat_reach: unit_data.combat_reach,
        bounding_radius: unit_data.bounding_radius,
    })
}
