// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Creature AI — state machine for NPC/mob behavior.
//!
//! Implements idle wandering, random movement, aggro detection, and
//! basic melee combat for server-controlled creatures.

use std::time::Instant;

use wow_core::{ObjectGuid, Position};
use wow_instances::BossAiRef;

// ── CreatureAISelector ────────────────────────────────────────────

/// Represented result of TrinityCore `FactorySelector::SelectAI(Creature*)`.
///
/// This is selector evidence only: no virtual AI object is instantiated and no
/// hooks are executed here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreatureAiKindLikeCpp {
    PetAI,
    ScriptedAI(String),
    NullCreatureAI,
    TriggerAI,
    AggressorAI,
    ReactorAI,
    PassiveAI,
    PossessedAI,
    CritterAI,
    GuardAI,
    TotemAI,
    CombatAI,
    TurretAI,
    VehicleAI,
    SmartAI,
    ScheduledChangeAI,
    UnknownNamedAI(String),
}

/// Minimal, already-resolved C++ creature facts needed by the stock selector.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CreatureAiSelectionInputLikeCpp {
    pub ai_name: String,
    pub script_name: String,
    pub script_can_create_creature_ai: bool,
    pub is_pet: bool,
    pub is_vehicle: bool,
    pub is_totem: bool,
    pub is_trigger: bool,
    pub first_spell_id: u32,
    pub is_critter: bool,
    pub is_guardian: bool,
    pub is_guard: bool,
    pub is_civilian: bool,
    pub is_neutral_to_all: bool,
    pub has_spellclick_npc_flag: bool,
    pub is_controllable_guardian: bool,
    pub controllable_guardian_owner_is_player: bool,
}

impl CreatureAiKindLikeCpp {
    fn from_registered_ai_name_like_cpp(ai_name: &str) -> Self {
        match ai_name {
            "NullCreatureAI" => Self::NullCreatureAI,
            "TriggerAI" => Self::TriggerAI,
            "AggressorAI" => Self::AggressorAI,
            "ReactorAI" => Self::ReactorAI,
            "PassiveAI" => Self::PassiveAI,
            "PossessedAI" => Self::PossessedAI,
            "CritterAI" => Self::CritterAI,
            "GuardAI" => Self::GuardAI,
            "PetAI" => Self::PetAI,
            "TotemAI" => Self::TotemAI,
            "CombatAI" => Self::CombatAI,
            "TurretAI" => Self::TurretAI,
            "VehicleAI" => Self::VehicleAI,
            "SmartAI" => Self::SmartAI,
            "ScheduledChangeAI" => Self::ScheduledChangeAI,
            other => Self::UnknownNamedAI(other.to_string()),
        }
    }
}

pub fn select_creature_ai_like_cpp(
    input: &CreatureAiSelectionInputLikeCpp,
) -> CreatureAiKindLikeCpp {
    // C++ `FactorySelector::SelectAI`: pet override happens before DB ScriptName
    // and AIName so tamed creatures cannot keep a template SmartAI.
    if input.is_pet {
        return CreatureAiKindLikeCpp::PetAI;
    }

    if input.script_can_create_creature_ai && !input.script_name.is_empty() {
        return CreatureAiKindLikeCpp::ScriptedAI(input.script_name.clone());
    }

    if !input.ai_name.is_empty() {
        return CreatureAiKindLikeCpp::from_registered_ai_name_like_cpp(&input.ai_name);
    }

    select_creature_ai_by_permit_like_cpp(input)
}

fn select_creature_ai_by_permit_like_cpp(
    input: &CreatureAiSelectionInputLikeCpp,
) -> CreatureAiKindLikeCpp {
    // C++ iterates ObjectRegistry's std::map and picks max permit; equal permits
    // keep the first lexicographic AIName because std::max_element is stable for
    // equivalent values.
    [
        ("AggressorAI", permit_aggressor_ai_like_cpp(input)),
        ("CombatAI", -1),
        ("CritterAI", permit_critter_ai_like_cpp(input)),
        ("GuardAI", permit_guard_ai_like_cpp(input)),
        ("NullCreatureAI", permit_null_creature_ai_like_cpp(input)),
        ("PassiveAI", -1),
        ("PetAI", permit_pet_ai_like_cpp(input)),
        ("PossessedAI", -1),
        ("ReactorAI", permit_reactor_ai_like_cpp(input)),
        ("ScheduledChangeAI", -1),
        ("SmartAI", -1),
        ("TotemAI", permit_totem_ai_like_cpp(input)),
        ("TriggerAI", permit_trigger_ai_like_cpp(input)),
        ("TurretAI", -1),
        ("VehicleAI", permit_vehicle_ai_like_cpp(input)),
    ]
    .into_iter()
    .filter(|(_, permit)| *permit >= 0)
    .fold(None::<(&str, i32)>, |selected, candidate| match selected {
        Some(current) if current.1 >= candidate.1 => Some(current),
        _ => Some(candidate),
    })
    .map(|(ai_name, _)| CreatureAiKindLikeCpp::from_registered_ai_name_like_cpp(ai_name))
    .unwrap_or(CreatureAiKindLikeCpp::NullCreatureAI)
}

fn permit_aggressor_ai_like_cpp(input: &CreatureAiSelectionInputLikeCpp) -> i32 {
    if !input.is_civilian && !input.is_neutral_to_all {
        100
    } else {
        -1
    }
}

fn permit_critter_ai_like_cpp(input: &CreatureAiSelectionInputLikeCpp) -> i32 {
    if input.is_critter && !input.is_guardian {
        200
    } else {
        -1
    }
}

fn permit_guard_ai_like_cpp(input: &CreatureAiSelectionInputLikeCpp) -> i32 {
    if input.is_guard { 200 } else { -1 }
}

fn permit_null_creature_ai_like_cpp(input: &CreatureAiSelectionInputLikeCpp) -> i32 {
    if input.has_spellclick_npc_flag {
        250
    } else if input.is_trigger {
        200
    } else {
        1
    }
}

fn permit_pet_ai_like_cpp(input: &CreatureAiSelectionInputLikeCpp) -> i32 {
    if input.is_controllable_guardian {
        if input.controllable_guardian_owner_is_player {
            200
        } else {
            100
        }
    } else {
        -1
    }
}

fn permit_reactor_ai_like_cpp(input: &CreatureAiSelectionInputLikeCpp) -> i32 {
    if input.is_civilian || input.is_neutral_to_all {
        100
    } else {
        -1
    }
}

fn permit_totem_ai_like_cpp(input: &CreatureAiSelectionInputLikeCpp) -> i32 {
    if input.is_totem { 200 } else { -1 }
}

fn permit_trigger_ai_like_cpp(input: &CreatureAiSelectionInputLikeCpp) -> i32 {
    if input.is_trigger && input.first_spell_id != 0 {
        800
    } else {
        -1
    }
}

fn permit_vehicle_ai_like_cpp(input: &CreatureAiSelectionInputLikeCpp) -> i32 {
    if input.is_vehicle { 800 } else { -1 }
}

// ── CreatureAI::CanAIAttack ───────────────────────────────────────

/// Already-resolved facts for represented `AI()->CanAIAttack(target)`.
///
/// Most stock C++ creature AIs inherit `UnitAI::CanAIAttack == true`; this
/// input carries only the extra facts needed by represented overrides.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreatureAiCanAttackInputLikeCpp {
    pub target_within_turret_combat_range: bool,
    pub target_within_turret_min_range: bool,
    pub boss_boundary_contains_target: Option<bool>,
}

impl Default for CreatureAiCanAttackInputLikeCpp {
    fn default() -> Self {
        Self {
            target_within_turret_combat_range: true,
            target_within_turret_min_range: false,
            boss_boundary_contains_target: None,
        }
    }
}

pub fn creature_ai_can_attack_like_cpp(
    ai_kind: &CreatureAiKindLikeCpp,
    input: &CreatureAiCanAttackInputLikeCpp,
) -> bool {
    // C++ `BossAI::CanAIAttack` is a script-provided AI override, not a stock
    // registry name. The caller must prove the selected script is BossAI before
    // passing this boundary fact.
    if let Some(in_boundary) = input.boss_boundary_contains_target {
        return in_boundary;
    }

    match ai_kind {
        CreatureAiKindLikeCpp::TurretAI => {
            input.target_within_turret_combat_range && !input.target_within_turret_min_range
        }
        _ => true,
    }
}

pub fn creature_ai_uses_base_move_in_line_of_sight_like_cpp(
    ai_kind: &CreatureAiKindLikeCpp,
) -> bool {
    match ai_kind {
        CreatureAiKindLikeCpp::NullCreatureAI
        | CreatureAiKindLikeCpp::TriggerAI
        | CreatureAiKindLikeCpp::ReactorAI
        | CreatureAiKindLikeCpp::PassiveAI
        | CreatureAiKindLikeCpp::PossessedAI
        | CreatureAiKindLikeCpp::CritterAI
        | CreatureAiKindLikeCpp::PetAI
        | CreatureAiKindLikeCpp::TotemAI
        | CreatureAiKindLikeCpp::VehicleAI
        | CreatureAiKindLikeCpp::ScheduledChangeAI => false,
        CreatureAiKindLikeCpp::ScriptedAI(_)
        | CreatureAiKindLikeCpp::UnknownNamedAI(_)
        | CreatureAiKindLikeCpp::AggressorAI
        | CreatureAiKindLikeCpp::GuardAI
        | CreatureAiKindLikeCpp::CombatAI
        | CreatureAiKindLikeCpp::TurretAI
        | CreatureAiKindLikeCpp::SmartAI => true,
    }
}

// ── CreatureState ──────────────────────────────────────────────────

/// Current AI state for a creature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatureState {
    /// Idle — creature stands still or wanders randomly.
    Idle,
    /// Walking to a random point within home radius.
    WalkingRandom,
    /// Walking along a waypoint path.
    WalkingWaypoint,
    /// In combat with a player.
    InCombat,
    /// Dead — waiting for respawn.
    Dead,
    /// Returning to spawn point after combat reset.
    Returning,
}

// ── CreatureAI ────────────────────────────────────────────────────

/// Server-side state for one spawned creature.
///
/// Tracks position, health, movement timers, and combat state.
/// The session update loop calls [`CreatureAI::update`] every tick.
#[derive(Debug, Clone)]
pub struct CreatureAI {
    pub guid: ObjectGuid,
    pub entry: u32,

    /// Spawn (home) position.
    pub home_pos: Position,
    /// Current position (updated as it moves).
    pub current_pos: Position,
    /// Destination for current movement (None if standing still).
    pub move_target: Option<Position>,
    /// When the current movement started.
    pub move_start: Instant,
    /// How long the current movement takes (ms).
    pub move_duration_ms: u32,
    /// Spline ID counter (incremented on each move command).
    pub spline_id: u32,

    /// Current AI state.
    pub state: CreatureState,
    /// Time until next random movement attempt.
    pub wander_timer: Instant,
    /// Wander delay before moving again (random 5–15s).
    pub wander_delay_ms: u64,

    /// Current HP.
    pub hp: u32,
    /// Max HP.
    pub max_hp: u32,

    /// Level (used for damage/aggro calculations).
    pub level: u8,
    /// Melee damage range.
    pub min_dmg: u32,
    pub max_dmg: u32,

    /// Current combat target (player GUID).
    pub combat_target: Option<ObjectGuid>,
    /// Last time this creature swung its weapon.
    pub last_swing: Instant,
    /// Swing timer in ms (base 2000ms for most mobs).
    pub swing_timer_ms: u64,

    /// Aggro radius (yards). Typical is 10–20.
    pub aggro_radius: f32,

    /// Maximum wander distance from home position (yards).
    pub wander_radius: f32,

    /// Whether this creature is alive.
    pub is_alive: bool,

    /// Time of death (for respawn logic).
    pub death_time: Option<Instant>,
    /// Respawn time in seconds.
    pub respawn_time_secs: u64,

    /// When this corpse should despawn from the world (set after fully looted).
    /// None = corpse never explicitly looted / not yet triggered.
    ///
    /// C# ref: `AllLootRemovedFromCorpse()` sets `m_corpseRemoveTime`.
    pub corpse_despawn_at: Option<Instant>,

    /// NPC flags (vendor, quest giver, etc.) — stored here for convenience.
    pub npc_flags: u32,
    /// Unit flags.
    pub unit_flags: u32,
    /// Display ID.
    pub display_id: u32,
    /// Faction template ID.
    pub faction: u32,
    /// Resolved C++ `Creature::GetLootId()` value for corpse loot.
    pub loot_id: u32,
    /// C++ `CreatureDifficulty::GoldMin`.
    pub gold_min: u32,
    /// C++ `CreatureDifficulty::GoldMax`.
    pub gold_max: u32,
    /// Represented C++ `BossAI::_bossId`, if this creature uses BossAI.
    pub boss_id: Option<u32>,
    /// Represented C++ `Loot::_dungeonEncounterId` source for corpse loot.
    pub dungeon_encounter_id: u32,
}

impl CreatureAI {
    /// Create a new creature AI with default idle state.
    pub fn new(
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
        loot_id: u32,
        gold_min: u32,
        gold_max: u32,
        boss_id: Option<u32>,
        dungeon_encounter_id: u32,
    ) -> Self {
        let now = Instant::now();
        // Derive rough damage if zero
        let (min_dmg, max_dmg) = if min_dmg == 0 {
            let base = (level as u32) * 3 + 5;
            (base, base + base / 2)
        } else {
            (min_dmg, max_dmg)
        };
        Self {
            guid,
            entry,
            home_pos: pos.clone(),
            current_pos: pos,
            move_target: None,
            move_start: now,
            move_duration_ms: 0,
            spline_id: 1,
            state: CreatureState::Idle,
            wander_timer: now,
            wander_delay_ms: 8_000,
            hp,
            max_hp: hp,
            level,
            min_dmg,
            max_dmg,
            combat_target: None,
            last_swing: now,
            swing_timer_ms: 2_000,
            aggro_radius,
            wander_radius: 5.0,
            is_alive: true,
            death_time: None,
            respawn_time_secs: 30,
            corpse_despawn_at: None,
            npc_flags,
            unit_flags,
            display_id,
            faction,
            loot_id,
            gold_min,
            gold_max,
            boss_id,
            dungeon_encounter_id,
        }
    }

    /// Returns true if this creature can wander randomly.
    /// Creatures with vendors, quest givers etc. typically don't wander.
    pub fn can_wander(&self) -> bool {
        // npc_flags: 1=gossip, 2=quest giver, 4=unk, 8=vendor, 16=trainer...
        // creatures with certain flags stay put
        self.npc_flags == 0 || (self.npc_flags & 0x80) == 0 // no UNIT_NPC_FLAG_INNKEEPER etc
    }

    /// Try to engage a player in combat.
    ///
    /// Returns true if the creature enters combat (was idle, player in range).
    pub fn try_aggro(&mut self, player_guid: ObjectGuid, player_pos: &Position) -> bool {
        if !self.is_alive || self.state == CreatureState::InCombat {
            return false;
        }
        let dist = self.current_pos.distance(player_pos);
        if dist <= self.aggro_radius {
            self.enter_combat(player_guid);
            return true;
        }
        false
    }

    /// Enter combat with a specific player.
    pub fn enter_combat(&mut self, player_guid: ObjectGuid) {
        self.state = CreatureState::InCombat;
        self.combat_target = Some(player_guid);
        self.move_target = None;
    }

    /// Leave combat and return to home position.
    pub fn reset_combat(&mut self) {
        self.state = CreatureState::Returning;
        self.combat_target = None;
        self.hp = self.max_hp;
        self.move_target = Some(self.home_pos.clone());
    }

    /// Apply damage to the creature.
    ///
    /// Returns true if the creature just died.
    pub fn take_damage(&mut self, dmg: u32) -> bool {
        if !self.is_alive {
            return false;
        }
        self.hp = self.hp.saturating_sub(dmg);
        if self.hp == 0 {
            self.die();
            return true;
        }
        false
    }

    /// Kill the creature.
    pub fn die(&mut self) {
        self.is_alive = false;
        self.state = CreatureState::Dead;
        self.combat_target = None;
        self.death_time = Some(Instant::now());
    }

    /// Check if the creature should respawn.
    pub fn should_respawn(&self) -> bool {
        if let Some(dt) = self.death_time {
            dt.elapsed().as_secs() >= self.respawn_time_secs
        } else {
            false
        }
    }

    /// Respawn the creature at its home position.
    pub fn respawn(&mut self) {
        self.hp = self.max_hp;
        self.is_alive = true;
        self.state = CreatureState::Idle;
        self.current_pos = self.home_pos.clone();
        self.move_target = None;
        self.death_time = None;
        self.spline_id += 1;
        self.wander_timer = Instant::now();
    }

    /// Check if the creature's current movement is complete.
    pub fn movement_finished(&self) -> bool {
        if self.move_target.is_none() {
            return true;
        }
        self.move_start.elapsed().as_millis() as u32 >= self.move_duration_ms
    }

    /// Interpolate the creature's current position along its movement path.
    pub fn interpolated_position(&self) -> Position {
        let Some(ref dst) = self.move_target else {
            return self.current_pos.clone();
        };
        let elapsed = self.move_start.elapsed().as_millis() as f32;
        let total = self.move_duration_ms as f32;
        if total <= 0.0 {
            return dst.clone();
        }
        let t = (elapsed / total).min(1.0);
        Position::new(
            self.current_pos.x + (dst.x - self.current_pos.x) * t,
            self.current_pos.y + (dst.y - self.current_pos.y) * t,
            self.current_pos.z + (dst.z - self.current_pos.z) * t,
            dst.orientation,
        )
    }

    /// Begin a move to the destination at walk speed (2.5 y/s).
    pub fn begin_move(&mut self, dst: Position) {
        let dist = self.current_pos.distance(&dst);
        let walk_speed = 2.5f32;
        let duration_ms = ((dist / walk_speed) * 1000.0) as u32;
        self.move_target = Some(dst);
        self.move_start = Instant::now();
        self.move_duration_ms = duration_ms.max(500);
        self.spline_id += 1;
    }

    /// Finalize movement — snap to destination.
    pub fn finish_move(&mut self) {
        if let Some(dst) = self.move_target.take() {
            self.current_pos = dst;
        }
        self.move_duration_ms = 0;
    }

    /// Check if it's time to swing the weapon.
    pub fn can_swing(&self) -> bool {
        self.is_alive
            && self.state == CreatureState::InCombat
            && self.last_swing.elapsed().as_millis() as u64 >= self.swing_timer_ms
    }

    /// Record that a swing happened.
    pub fn record_swing(&mut self) {
        self.last_swing = Instant::now();
    }

    /// Roll a random damage value in [min_dmg, max_dmg].
    pub fn roll_damage(&self) -> u32 {
        if self.min_dmg >= self.max_dmg {
            return self.min_dmg;
        }
        let range = self.max_dmg - self.min_dmg;
        // Simple LCG-style pseudo-random based on timer
        let seed = self.last_swing.elapsed().subsec_nanos();
        self.min_dmg + (seed % (range + 1))
    }

    /// Check if creature should check wander movement.
    pub fn should_wander(&self) -> bool {
        self.is_alive
            && self.state == CreatureState::Idle
            && self.can_wander()
            && self.wander_timer.elapsed().as_millis() as u64 >= self.wander_delay_ms
    }

    /// Pick a random wander destination near home.
    pub fn pick_wander_destination(&mut self) -> Position {
        // Simple pseudo-random using elapsed time as seed
        let seed = self.wander_timer.elapsed().subsec_nanos() as f32;
        let angle = (seed * 0.001) % (2.0 * std::f32::consts::PI);
        let dist = (seed * 0.0001) % self.wander_radius + 1.0;
        let x = self.home_pos.x + angle.cos() * dist;
        let y = self.home_pos.y + angle.sin() * dist;
        let o = angle + std::f32::consts::PI; // face movement direction
        Position::new(x, y, self.home_pos.z, o)
    }

    /// Reset the wander timer with a random delay.
    pub fn reset_wander_timer(&mut self) {
        self.wander_timer = Instant::now();
        // Random delay 5–15 seconds
        let seed = self.wander_timer.elapsed().subsec_nanos() as u64;
        self.wander_delay_ms = 5_000 + (seed % 10_000);
    }
}

impl CreatureAI {
    /// Return the represented C++ `BossAI` view only when this creature has
    /// script-provided boss identity. A plain creature must behave like a
    /// failed `dynamic_cast<BossAI const*>`.
    pub fn boss_ai_like_cpp(&self) -> Option<BossAiRef> {
        self.boss_id.map(BossAiRef::new)
    }
}

// ── Position distance helper ──────────────────────────────────────
// Position already has .distance() from wow-core; we define this
// convenience method here for internal use.

fn position_dist(a: &Position, b: &Position) -> f32 {
    a.distance(b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_instances::BossAiLikeCpp;

    fn selector_input() -> CreatureAiSelectionInputLikeCpp {
        CreatureAiSelectionInputLikeCpp::default()
    }

    #[test]
    fn creature_ai_selector_pet_overrides_script_and_ai_name_like_cpp() {
        let input = CreatureAiSelectionInputLikeCpp {
            is_pet: true,
            script_name: "boss_should_not_win".to_string(),
            script_can_create_creature_ai: true,
            ai_name: "SmartAI".to_string(),
            ..selector_input()
        };

        assert_eq!(
            select_creature_ai_like_cpp(&input),
            CreatureAiKindLikeCpp::PetAI
        );
    }

    #[test]
    fn creature_ai_selector_uses_script_before_ai_name_like_cpp() {
        let input = CreatureAiSelectionInputLikeCpp {
            script_name: "npc_scripted".to_string(),
            script_can_create_creature_ai: true,
            ai_name: "AggressorAI".to_string(),
            ..selector_input()
        };

        assert_eq!(
            select_creature_ai_like_cpp(&input),
            CreatureAiKindLikeCpp::ScriptedAI("npc_scripted".to_string())
        );
    }

    #[test]
    fn creature_ai_selector_uses_registered_ai_name_before_permits_like_cpp() {
        let input = CreatureAiSelectionInputLikeCpp {
            ai_name: "TurretAI".to_string(),
            is_vehicle: true,
            ..selector_input()
        };

        assert_eq!(
            select_creature_ai_like_cpp(&input),
            CreatureAiKindLikeCpp::TurretAI
        );
    }

    #[test]
    fn creature_ai_selector_falls_back_to_stock_permits_like_cpp() {
        assert_eq!(
            select_creature_ai_like_cpp(&CreatureAiSelectionInputLikeCpp {
                is_vehicle: true,
                ..selector_input()
            }),
            CreatureAiKindLikeCpp::VehicleAI
        );
        assert_eq!(
            select_creature_ai_like_cpp(&CreatureAiSelectionInputLikeCpp {
                is_trigger: true,
                first_spell_id: 133,
                is_vehicle: true,
                ..selector_input()
            }),
            CreatureAiKindLikeCpp::TriggerAI
        );
        assert_eq!(
            select_creature_ai_like_cpp(&CreatureAiSelectionInputLikeCpp {
                is_trigger: true,
                ..selector_input()
            }),
            CreatureAiKindLikeCpp::NullCreatureAI
        );
        assert_eq!(
            select_creature_ai_like_cpp(&CreatureAiSelectionInputLikeCpp {
                has_spellclick_npc_flag: true,
                is_guard: true,
                ..selector_input()
            }),
            CreatureAiKindLikeCpp::NullCreatureAI
        );
        assert_eq!(
            select_creature_ai_like_cpp(&CreatureAiSelectionInputLikeCpp {
                is_guard: true,
                ..selector_input()
            }),
            CreatureAiKindLikeCpp::GuardAI
        );
        assert_eq!(
            select_creature_ai_like_cpp(&CreatureAiSelectionInputLikeCpp {
                is_controllable_guardian: true,
                ..selector_input()
            }),
            CreatureAiKindLikeCpp::AggressorAI
        );
        assert_eq!(
            select_creature_ai_like_cpp(&CreatureAiSelectionInputLikeCpp {
                is_controllable_guardian: true,
                is_civilian: true,
                ..selector_input()
            }),
            CreatureAiKindLikeCpp::PetAI
        );
        assert_eq!(
            select_creature_ai_like_cpp(&CreatureAiSelectionInputLikeCpp {
                is_civilian: true,
                ..selector_input()
            }),
            CreatureAiKindLikeCpp::ReactorAI
        );
        assert_eq!(
            select_creature_ai_like_cpp(&CreatureAiSelectionInputLikeCpp {
                is_neutral_to_all: true,
                ..selector_input()
            }),
            CreatureAiKindLikeCpp::ReactorAI
        );
        assert_eq!(
            select_creature_ai_like_cpp(&selector_input()),
            CreatureAiKindLikeCpp::AggressorAI
        );
    }

    #[test]
    fn creature_ai_selector_preserves_unknown_ai_name_as_unrepresented_like_cpp() {
        let input = CreatureAiSelectionInputLikeCpp {
            ai_name: "CustomPrivateAI".to_string(),
            ..selector_input()
        };

        assert_eq!(
            select_creature_ai_like_cpp(&input),
            CreatureAiKindLikeCpp::UnknownNamedAI("CustomPrivateAI".to_string())
        );
    }

    #[test]
    fn creature_ai_can_attack_defaults_true_for_stock_ai_like_cpp() {
        for ai_kind in [
            CreatureAiKindLikeCpp::AggressorAI,
            CreatureAiKindLikeCpp::ReactorAI,
            CreatureAiKindLikeCpp::GuardAI,
            CreatureAiKindLikeCpp::SmartAI,
            CreatureAiKindLikeCpp::VehicleAI,
            CreatureAiKindLikeCpp::UnknownNamedAI("CustomPrivateAI".to_string()),
        ] {
            assert!(
                creature_ai_can_attack_like_cpp(
                    &ai_kind,
                    &CreatureAiCanAttackInputLikeCpp {
                        target_within_turret_combat_range: false,
                        target_within_turret_min_range: true,
                        boss_boundary_contains_target: None,
                    },
                ),
                "{ai_kind:?} should inherit UnitAI::CanAIAttack == true"
            );
        }
    }

    #[test]
    fn creature_ai_can_attack_applies_turret_range_override_like_cpp() {
        assert!(creature_ai_can_attack_like_cpp(
            &CreatureAiKindLikeCpp::TurretAI,
            &CreatureAiCanAttackInputLikeCpp {
                target_within_turret_combat_range: true,
                target_within_turret_min_range: false,
                boss_boundary_contains_target: None,
            },
        ));
        assert!(!creature_ai_can_attack_like_cpp(
            &CreatureAiKindLikeCpp::TurretAI,
            &CreatureAiCanAttackInputLikeCpp {
                target_within_turret_combat_range: false,
                target_within_turret_min_range: false,
                boss_boundary_contains_target: None,
            },
        ));
        assert!(!creature_ai_can_attack_like_cpp(
            &CreatureAiKindLikeCpp::TurretAI,
            &CreatureAiCanAttackInputLikeCpp {
                target_within_turret_combat_range: true,
                target_within_turret_min_range: true,
                boss_boundary_contains_target: None,
            },
        ));
    }

    #[test]
    fn creature_ai_can_attack_applies_boss_boundary_override_like_cpp() {
        assert!(!creature_ai_can_attack_like_cpp(
            &CreatureAiKindLikeCpp::ScriptedAI("boss_script".to_string()),
            &CreatureAiCanAttackInputLikeCpp {
                boss_boundary_contains_target: Some(false),
                ..CreatureAiCanAttackInputLikeCpp::default()
            },
        ));
        assert!(creature_ai_can_attack_like_cpp(
            &CreatureAiKindLikeCpp::ScriptedAI("boss_script".to_string()),
            &CreatureAiCanAttackInputLikeCpp {
                boss_boundary_contains_target: Some(true),
                target_within_turret_combat_range: false,
                target_within_turret_min_range: true,
            },
        ));
    }

    #[test]
    fn creature_ai_move_in_line_of_sight_empty_overrides_are_suppressed_like_cpp() {
        for ai_kind in [
            CreatureAiKindLikeCpp::NullCreatureAI,
            CreatureAiKindLikeCpp::TriggerAI,
            CreatureAiKindLikeCpp::ReactorAI,
            CreatureAiKindLikeCpp::PassiveAI,
            CreatureAiKindLikeCpp::PossessedAI,
            CreatureAiKindLikeCpp::CritterAI,
            CreatureAiKindLikeCpp::PetAI,
            CreatureAiKindLikeCpp::TotemAI,
            CreatureAiKindLikeCpp::VehicleAI,
            CreatureAiKindLikeCpp::ScheduledChangeAI,
        ] {
            assert!(
                !creature_ai_uses_base_move_in_line_of_sight_like_cpp(&ai_kind),
                "{ai_kind:?} overrides MoveInLineOfSight with no base auto-aggro"
            );
        }
    }

    #[test]
    fn creature_ai_move_in_line_of_sight_base_users_keep_auto_aggro_path_like_cpp() {
        for ai_kind in [
            CreatureAiKindLikeCpp::AggressorAI,
            CreatureAiKindLikeCpp::GuardAI,
            CreatureAiKindLikeCpp::CombatAI,
            CreatureAiKindLikeCpp::TurretAI,
            CreatureAiKindLikeCpp::SmartAI,
            CreatureAiKindLikeCpp::ScriptedAI("npc_scripted".to_string()),
            CreatureAiKindLikeCpp::UnknownNamedAI("CustomAI".to_string()),
        ] {
            assert!(
                creature_ai_uses_base_move_in_line_of_sight_like_cpp(&ai_kind),
                "{ai_kind:?} should reach the base MoveInLineOfSight aggro path"
            );
        }
    }

    fn creature_with_boss_id(boss_id: Option<u32>) -> CreatureAI {
        CreatureAI::new(
            ObjectGuid::EMPTY,
            1,
            Position::ZERO,
            100,
            1,
            1,
            2,
            0.0,
            1,
            35,
            0,
            0,
            0,
            0,
            0,
            boss_id,
            0,
        )
    }

    #[test]
    fn plain_creature_has_no_boss_ai_view_like_cpp_failed_dynamic_cast() {
        let creature = creature_with_boss_id(None);

        assert!(creature.boss_ai_like_cpp().is_none());
    }

    #[test]
    fn boss_creature_exposes_script_boss_id_like_cpp_boss_ai() {
        let creature = creature_with_boss_id(Some(7));

        assert_eq!(creature.boss_ai_like_cpp().unwrap().boss_id(), 7);
    }
}
