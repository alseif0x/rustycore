//! One synchronous loot motor for either legitimate WorldCreature owner.
//! C++ a5f8da2e Unit::Kill (10457), Creature::AllLootRemovedFromCorpse (2942)
//! and LootHandler::DoLootRelease (270). No clocks/IO are added by this bridge.
use super::WorldCreature;
use std::collections::HashMap;
use wow_core::{ObjectGuid, Position};
use wow_entities::{CreatureLoot, OwnedLootAuthority, UnitValuesUpdate};

#[derive(Debug)]
pub struct CreatureLootObservation {
    is_alive: bool,
    position: Position,
    level: u8,
    entry: u32,
    loot_id: u32,
    gold_min: u32,
    gold_max: u32,
    dungeon_encounter_id: u32,
    tappers: Vec<ObjectGuid>,
    loot_lifecycle_revision: u64,
}

impl CreatureLootObservation {
    pub fn is_alive(&self) -> bool {
        self.is_alive
    }
    pub fn position(&self) -> Position {
        self.position
    }
    pub fn level(&self) -> u8 {
        self.level
    }
    pub fn entry(&self) -> u32 {
        self.entry
    }
    pub fn loot_id(&self) -> u32 {
        self.loot_id
    }
    pub fn gold_min(&self) -> u32 {
        self.gold_min
    }
    pub fn gold_max(&self) -> u32 {
        self.gold_max
    }
    pub fn dungeon_encounter_id(&self) -> u32 {
        self.dungeon_encounter_id
    }
    pub fn tappers(&self) -> &[ObjectGuid] {
        &self.tappers
    }
    pub fn loot_lifecycle_revision(&self) -> u64 {
        self.loot_lifecycle_revision
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatureLootReleasePhase {
    Normal,
    Detached,
}

#[derive(Debug)]
pub struct CreatureLootReleaseOutcome {
    marked: Option<(u32, u32)>,
    values: UnitValuesUpdate,
}
impl CreatureLootReleaseOutcome {
    pub fn into_parts(self) -> (Option<(u32, u32)>, UnitValuesUpdate) {
        (self.marked, self.values)
    }
}

impl WorldCreature {
    /// This read never copies an entity or rebuilds its runtime.
    pub fn observe_loot(&self) -> CreatureLootObservation {
        CreatureLootObservation {
            is_alive: self.is_alive(),
            position: self.position(),
            level: self.level(),
            entry: self.entry(),
            loot_id: self.loot_id(),
            gold_min: self.gold_min(),
            gold_max: self.gold_max(),
            dungeon_encounter_id: self.dungeon_encounter_id(),
            tappers: self.creature.tap_list().to_vec(),
            loot_lifecycle_revision: self.creature.loot_lifecycle_revision_like_cpp(),
        }
    }

    /// Preserve the original predicate and mutation order, including generation zero.
    pub fn install_kill_loot(
        &mut self,
        expected_authority: &OwnedLootAuthority,
        object_generation: u64,
        creature_lifetime: u64,
        shared: Option<CreatureLoot>,
        personal: HashMap<ObjectGuid, CreatureLoot>,
    ) -> bool {
        if self.is_alive()
            || self.creature.loot_lifecycle_revision_like_cpp() != creature_lifetime
            || !self
                .creature
                .loot_authority_like_cpp()
                .shares_storage_like_cpp(expected_authority)
            || !expected_authority.is_retired_like_cpp()
            || expected_authority.generation_like_cpp() != object_generation
        {
            return false;
        }
        let installed = if object_generation == 0 {
            expected_authority
                .initialize_pristine_like_cpp(shared, personal)
                .installed()
        } else {
            expected_authority
                .replace_retired_generation_like_cpp(object_generation, shared, personal)
                .is_some()
        };
        if installed {
            self.creature.sync_loot_summaries_from_authority_like_cpp();
        }
        installed
    }

    /// Normal release calls this BEFORE its whole-owner lifecycle guard.
    pub fn force_loot_flags(&mut self) -> UnitValuesUpdate {
        self.force_dynamic_flags_update_like_cpp();
        self.creature.unit().values_update()
    }

    /// The caller holds the owning map guard; the authority callback never reenters it.
    pub fn release_looted_corpse(
        &mut self,
        authority: &OwnedLootAuthority,
        object_generation: u64,
        authority_lifecycle_revision: u64,
        fully_skinned: bool,
        decay_rate: f32,
        phase: CreatureLootReleasePhase,
    ) -> Option<CreatureLootReleaseOutcome> {
        self.with_loot_release_guard(
            authority,
            object_generation,
            authority_lifecycle_revision,
            phase,
            |actor| actor.finish_looted_corpse(fully_skinned, decay_rate, phase),
        )
    }

    /// Existing compatibility snapshots are captured INSIDE the authority guard,
    /// exactly where the former legacy mutator cloned its inner Creature.
    pub fn release_legacy_looted_corpse(
        &mut self,
        authority: &OwnedLootAuthority,
        object_generation: u64,
        authority_lifecycle_revision: u64,
        fully_skinned: bool,
        decay_rate: f32,
        phase: CreatureLootReleasePhase,
    ) -> Option<(CreatureLootReleaseOutcome, wow_entities::Creature)> {
        self.with_loot_release_guard(
            authority,
            object_generation,
            authority_lifecycle_revision,
            phase,
            |actor| {
                let result = actor.finish_looted_corpse(fully_skinned, decay_rate, phase);
                (result, actor.creature.clone())
            },
        )
    }

    fn with_loot_release_guard<R>(
        &mut self,
        authority: &OwnedLootAuthority,
        object_generation: u64,
        revision: u64,
        phase: CreatureLootReleasePhase,
        apply: impl FnOnce(&mut WorldCreature) -> R,
    ) -> Option<R> {
        if !self
            .creature
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(authority)
        {
            return None;
        }
        let apply = || apply(self);
        match phase {
            CreatureLootReleasePhase::Normal => authority
                .with_fully_looted_lifecycle_observation_like_cpp(
                    object_generation,
                    revision,
                    apply,
                ),
            CreatureLootReleasePhase::Detached => authority
                .with_unviewed_fully_looted_lifecycle_observation_like_cpp(
                    object_generation,
                    revision,
                    apply,
                ),
        }
    }

    /// Compatibility for the existing local fixture path, selected only with no Actor.
    pub fn release_local_looted_corpse(
        &mut self,
        fully_skinned: bool,
        decay_rate: f32,
    ) -> CreatureLootReleaseOutcome {
        self.finish_looted_corpse(fully_skinned, decay_rate, CreatureLootReleasePhase::Normal)
    }

    fn finish_looted_corpse(
        &mut self,
        fully_skinned: bool,
        decay_rate: f32,
        phase: CreatureLootReleasePhase,
    ) -> CreatureLootReleaseOutcome {
        if phase == CreatureLootReleasePhase::Detached {
            self.force_dynamic_flags_update_like_cpp();
        }
        self.remove_lootable_dynamic_flag_like_cpp();
        let marked = if !self.is_alive() {
            let corpse_decay_secs = wow_entities::looted_corpse_decay_seconds(
                fully_skinned,
                self.corpse_delay_secs_like_cpp(),
                self.ignore_corpse_decay_ratio_like_cpp(),
                decay_rate,
            );
            // The existing method samples its clocks at the same guarded call point.
            // Expired corpses retain the earlier flag writes and return no marker.
            if self.all_loot_removed_from_corpse_like_cpp(decay_rate, fully_skinned) {
                Some((self.entry(), corpse_decay_secs))
            } else {
                None
            }
        } else {
            None
        };
        CreatureLootReleaseOutcome {
            marked,
            values: self.creature.unit().values_update(),
        }
    }
}
#[cfg(test)]
mod tests;
