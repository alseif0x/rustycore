//! Data retained by a cast or a queued player request, without packet serialization.
//! C++: Spell.h / SpellCastTargets and Player::_pendingSpellCastRequest.

use std::time::Instant;
use wow_core::{ObjectGuid, Position};

/// Additional spell cast metadata that C++ stores on `Spell` before `prepare`.
///
/// Default values preserve the represented normal-cast path: `OriginalCastID`
/// is the same as `CastID`, `CastFlagsEx` is zero, and no item entry/misc data
/// is attached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellCastBattlePetItemModifiersLikeCpp {
    /// Stable identity of the caged item consumed by C++
    /// `SPELL_EFFECT_UNCAGE_BATTLEPET`.
    pub source_item_guid: ObjectGuid,
    pub species_id: u32,
    pub breed_data: u32,
    pub level: u16,
    pub display_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellCastMetadata {
    pub from_client: bool,
    /// Overrides the visible/effect caster for represented triggered casts.
    /// Normal player casts leave this empty and use the logged-in player GUID.
    pub caster_guid_override: Option<ObjectGuid>,
    /// C++ `SpellCastData::CastFlags` for the emitted `SMSG_SPELL_GO`.
    pub cast_flags: u32,
    pub misc: [i32; 2],
    pub cast_item_entry: Option<u32>,
    pub cast_item_battle_pet_modifiers: Option<SpellCastBattlePetItemModifiersLikeCpp>,
    pub cast_flags_ex: u32,
    pub original_cast_id: ObjectGuid,
    pub unit_target_battle_pet_companion_guid: Option<ObjectGuid>,
    pub restore_last_spell_cast_time_on_power_failure: bool,
    pub previous_last_spell_cast_time_on_power_failure: Option<Instant>,
}

impl Default for SpellCastMetadata {
    fn default() -> Self {
        Self {
            from_client: false,
            caster_guid_override: None,
            cast_flags: 0,
            misc: [0, 0],
            cast_item_entry: None,
            cast_item_battle_pet_modifiers: None,
            cast_flags_ex: 0,
            original_cast_id: ObjectGuid::EMPTY,
            unit_target_battle_pet_companion_guid: None,
            restore_last_spell_cast_time_on_power_failure: false,
            previous_last_spell_cast_time_on_power_failure: None,
        }
    }
}

impl SpellCastMetadata {
    pub fn original_cast_id_or(self, cast_id: ObjectGuid) -> ObjectGuid {
        if self.original_cast_id.is_empty() {
            cast_id
        } else {
            self.original_cast_id
        }
    }
}

/// C++ SpellCastTargets location and optional transport-relative coordinates.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SpellCastLocationLikeCpp {
    pub transport: ObjectGuid,
    pub position: Position,
}

/// Retained target values; decoding and encoding belong to the packet adapter.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SpellCastTargetsLikeCpp {
    pub flags: u32,
    pub unit: ObjectGuid,
    pub item: ObjectGuid,
    pub src_location: Option<SpellCastLocationLikeCpp>,
    pub dst_location: Option<SpellCastLocationLikeCpp>,
    pub orientation: Option<f32>,
    pub map_id: Option<i32>,
    pub name: String,
}

/// C++ SpellDefines.h::SpellCastVisual plus retained legacy script-visual evidence.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpellCastVisualLikeCpp {
    pub spell_visual_id: u32,
    pub script_visual_id: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpellCastState {
    pub spell_id: i32,
    pub target_guid: ObjectGuid,
    pub target_data: SpellCastTargetsLikeCpp,
    pub cast_id: ObjectGuid,
    pub cast_start_time: Instant,
    pub cast_time_ms: u32,
    pub spell_visual: SpellCastVisualLikeCpp,
    pub metadata: SpellCastMetadata,
}

/// Existing represented cast execution policy, owned by Unit's spell subsystem.
/// Timestamp semantics are retained until convergence with full SpellHistory policy.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CastExecutionStateLikeCpp {
    pub active: Option<SpellCastState>,
    pub last_cast_time: Option<Instant>,
    pub last_cast_time_per_spell: std::collections::HashMap<i32, Instant>,
}

impl CastExecutionStateLikeCpp {
    /// Represented counterpart of C++ Unit::InterruptSpell. The caller selects
    /// the eligible slot/spell; `None` cancels any retained active cast.
    /// Pending player requests and packet publication are separate transitions.
    pub fn interrupt_active_cast(&mut self, spell_id: Option<i32>) -> bool {
        if !self
            .active
            .as_ref()
            .is_some_and(|cast| spell_id.is_none_or(|id| cast.spell_id == id))
        {
            return false;
        }
        self.active.take();
        true
    }

    pub fn remaining_cast_ms(&self) -> u32 {
        self.active.as_ref().map_or(0, |cast| {
            cast.cast_time_ms
                .saturating_sub(cast.cast_start_time.elapsed().as_millis() as u32)
        })
    }

    /// Retains the existing Instant-based cooldown policy; this is not a claim
    /// of complete C++ SpellHistory policy or diff-timer convergence.
    pub fn remaining_global_cooldown_ms(&self, cooldown_ms: u32) -> u32 {
        self.last_cast_time.map_or(0, |last| {
            cooldown_ms.saturating_sub(last.elapsed().as_millis() as u32)
        })
    }

    /// C++ Spell::update (PREPARING) executes only when its timer is ready.
    /// Consume once and retain the prior timestamp for the represented late
    /// power-failure rollback. The returned value is an execution outcome, not
    /// a copied live cast; effect execution and delivery belong to the caller.
    pub fn take_ready_cast(&mut self) -> Option<SpellCastState> {
        let elapsed_ms = self.active.as_ref()?.cast_start_time.elapsed().as_millis() as u32;
        self.take_ready_cast_after_elapsed(elapsed_ms, Instant::now)
    }

    fn take_ready_cast_after_elapsed(
        &mut self,
        elapsed_ms: u32,
        timestamp: impl FnOnce() -> Instant,
    ) -> Option<SpellCastState> {
        if elapsed_ms < self.active.as_ref()?.cast_time_ms {
            return None;
        }
        let mut cast = self.active.take()?;
        cast.metadata.restore_last_spell_cast_time_on_power_failure = true;
        cast.metadata.previous_last_spell_cast_time_on_power_failure = self.last_cast_time;
        self.last_cast_time = Some(timestamp());
        Some(cast)
    }
}

/// Represented player-caster queue payload; cancellation does not cancel the active cast.
#[derive(Debug, Clone, PartialEq)]
pub struct PendingSpellCastRequestLikeCpp {
    pub cast_id: ObjectGuid,
    pub spell_id: i32,
    pub casting_unit_guid: ObjectGuid,
    pub target_guid: ObjectGuid,
    pub target_data: SpellCastTargetsLikeCpp,
    pub spell_visual: SpellCastVisualLikeCpp,
    pub metadata: SpellCastMetadata,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn cast(start: Instant, cast_time_ms: u32) -> SpellCastState {
        SpellCastState {
            spell_id: 133,
            target_guid: ObjectGuid::create_player(1, 42),
            target_data: SpellCastTargetsLikeCpp {
                name: "retained target".into(),
                ..Default::default()
            },
            cast_id: ObjectGuid::new(6, 17),
            cast_start_time: start,
            cast_time_ms,
            spell_visual: SpellCastVisualLikeCpp {
                spell_visual_id: 12,
                script_visual_id: 23,
            },
            metadata: SpellCastMetadata::default(),
        }
    }

    #[test]
    fn readiness_preserves_unready_state_and_consumes_at_exact_boundary_once() {
        let stamp = Instant::now();
        let mut execution = CastExecutionStateLikeCpp {
            active: Some(cast(stamp, 1_500)),
            last_cast_time: Some(stamp - Duration::from_secs(5)),
            last_cast_time_per_spell: [(133, stamp)].into_iter().collect(),
        };
        let before = execution.clone();
        assert!(
            execution
                .take_ready_cast_after_elapsed(1_499, || panic!("unready cast must not stamp"))
                .is_none()
        );
        assert_eq!(execution, before);

        let ready = execution
            .take_ready_cast_after_elapsed(1_500, || stamp)
            .unwrap();
        let mut expected = before.active.unwrap();
        expected
            .metadata
            .restore_last_spell_cast_time_on_power_failure = true;
        expected
            .metadata
            .previous_last_spell_cast_time_on_power_failure = before.last_cast_time;
        assert_eq!(
            ready, expected,
            "retain every target, visual and cast field"
        );
        assert!(execution.active.is_none());
        assert_eq!(execution.last_cast_time, Some(stamp));
        assert_eq!(
            execution.last_cast_time_per_spell,
            before.last_cast_time_per_spell
        );
        let completed = execution.clone();
        assert!(
            execution
                .take_ready_cast_after_elapsed(1_501, || panic!("empty cast must not stamp"))
                .is_none()
        );
        assert_eq!(execution, completed);
    }

    #[test]
    fn instant_cast_is_ready_without_a_previous_timestamp() {
        let stamp = Instant::now();
        let mut execution = CastExecutionStateLikeCpp {
            active: Some(cast(stamp, 0)),
            ..Default::default()
        };
        let ready = execution
            .take_ready_cast_after_elapsed(0, || stamp)
            .unwrap();
        assert!(ready.metadata.restore_last_spell_cast_time_on_power_failure);
        assert_eq!(
            ready
                .metadata
                .previous_last_spell_cast_time_on_power_failure,
            None
        );
        assert_eq!(execution.last_cast_time, Some(stamp));
        assert!(execution.last_cast_time_per_spell.is_empty());
    }

    #[test]
    fn interruption_matches_exact_or_any_spell_without_erasing_timestamps() {
        let stamp = Instant::now();
        for selected in [None, Some(133)] {
            let mut execution = CastExecutionStateLikeCpp {
                active: Some(cast(stamp, 1_500)),
                last_cast_time: Some(stamp),
                last_cast_time_per_spell: [(133, stamp)].into_iter().collect(),
            };
            let before = execution.clone();
            assert!(!execution.interrupt_active_cast(Some(134)));
            assert_eq!(execution, before);
            assert!(execution.interrupt_active_cast(selected));
            assert!(!execution.interrupt_active_cast(selected));
            assert!(execution.active.is_none());
            assert_eq!(execution.last_cast_time, before.last_cast_time);
            assert_eq!(
                execution.last_cast_time_per_spell,
                before.last_cast_time_per_spell
            );
        }
    }

    #[test]
    fn absent_and_expired_cast_or_cooldown_report_zero_without_mutation() {
        let empty = CastExecutionStateLikeCpp::default();
        assert_eq!(empty.remaining_cast_ms(), 0);
        assert_eq!(empty.remaining_global_cooldown_ms(1_500), 0);
        let start = Instant::now() - Duration::from_secs(10);
        let execution = CastExecutionStateLikeCpp {
            active: Some(cast(start, 1_500)),
            last_cast_time: Some(start),
            ..Default::default()
        };
        let before = execution.clone();
        assert_eq!(execution.remaining_cast_ms(), 0);
        assert_eq!(execution.remaining_global_cooldown_ms(1_500), 0);
        assert_eq!(execution.remaining_global_cooldown_ms(0), 0);
        assert_eq!(execution, before);
    }
}
