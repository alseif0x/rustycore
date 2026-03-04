// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Creature AI — state machine for NPC/mob behavior.
//!
//! Implements idle wandering, random movement, aggro detection, and
//! basic melee combat for server-controlled creatures.

use std::time::{Duration, Instant};

use wow_core::{ObjectGuid, Position};

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
        if !self.is_alive { return false; }
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
        if self.move_target.is_none() { return true; }
        self.move_start.elapsed().as_millis() as u32 >= self.move_duration_ms
    }

    /// Interpolate the creature's current position along its movement path.
    pub fn interpolated_position(&self) -> Position {
        let Some(ref dst) = self.move_target else { return self.current_pos.clone(); };
        let elapsed = self.move_start.elapsed().as_millis() as f32;
        let total = self.move_duration_ms as f32;
        if total <= 0.0 { return dst.clone(); }
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
        if self.min_dmg >= self.max_dmg { return self.min_dmg; }
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

// ── Position distance helper ──────────────────────────────────────
// Position already has .distance() from wow-core; we define this
// convenience method here for internal use.

fn position_dist(a: &Position, b: &Position) -> f32 {
    a.distance(b)
}
