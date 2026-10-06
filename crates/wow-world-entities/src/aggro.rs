use std::collections::HashSet;

use wow_core::ObjectGuid;
use wow_data::SpellThreatEntryLikeCpp;
use wow_entities::{
    AuraThreatSnapshotLikeCpp as CanonicalThreatAuraSnapshotLikeCpp,
    RepresentedAuraEffectAmountLikeCpp,
};
use wow_map::ManagedMapInnerLikeCpp;
use wow_world_core::session::{HubMut, HubRef, begin_combat_ref_on_map_like_cpp};

use crate::WorldEntitiesState;

impl WorldEntitiesState {
    pub fn creature_aggro_radius_for_faction_template_like_cpp(
        &self,
        hub: HubRef<'_>,
        faction_template_id: u32,
        default_radius: f32,
    ) -> f32 {
        if hub
            .catalogs
            .creature_faction_template_is_neutral_to_all_like_cpp(faction_template_id)
        {
            0.0
        } else {
            default_radius
        }
    }

    pub fn canonical_creature_threat_value_like_cpp(
        &self,
        hub: HubRef<'_>,
        creature_guid: ObjectGuid,
        attacker_guid: ObjectGuid,
    ) -> Option<f32> {
        let map_key = hub.core.current_canonical_player_map_key_like_cpp()?;
        let manager = hub.core.canonical_map_manager.as_ref()?.clone();
        let manager = manager.lock().ok()?;
        let managed = manager.find_map(map_key.map_id, map_key.instance_id)?;
        creature_threat_value_on_map_like_cpp(managed.map(), creature_guid, attacker_guid)
    }

    pub fn mirror_canonical_creature_threat_from_attacker_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        creature_guid: ObjectGuid,
        attacker_guid: ObjectGuid,
        threat_value: f32,
    ) -> bool {
        let Some(map_key) = hub.core.current_canonical_player_map_key_like_cpp() else {
            return false;
        };
        let Some(manager) = hub.core.canonical_map_manager.as_ref().cloned() else {
            return false;
        };
        let Ok(mut manager) = manager.lock() else {
            return false;
        };
        let Some(managed) = manager.find_map_mut(map_key.map_id, map_key.instance_id) else {
            return false;
        };
        mirror_creature_threat_from_attacker_on_map_like_cpp(
            managed.map_mut(),
            creature_guid,
            attacker_guid,
            threat_value,
        )
    }

    /// C++ `Spell::HandleThreatSpells` additive threat before target-count
    /// distribution. Explicit `spell_threat` rows replace the SpellLevel
    /// fallback. This returns the unmodified flat/AP amount: the positive
    /// `ForwardThreatForAssistingMe` path applies spell modifiers, while the
    /// harmful path calls `AddThreat(..., ignoreModifiers = true)`.
    pub fn spell_initial_threat_like_cpp(
        &self,
        hub: HubRef<'_>,
        spell_id: u32,
        threat_entry: Option<SpellThreatEntryLikeCpp>,
        caster_guid: ObjectGuid,
    ) -> Option<f32> {
        if let Some(entry) = threat_entry {
            let caster_attack_power = if hub.core.player_guid() == Some(caster_guid) {
                hub.core
                    .canonical_player_total_attack_power_like_cpp()
                    .unwrap_or(0.0)
                    .max(0.0)
            } else {
                0.0
            };
            return Some(entry.flat_mod as f32 + entry.ap_pct_mod * caster_attack_power);
        }

        let difficulty = hub.core.current_map_difficulty_id_like_cpp();
        if hub
            .catalogs
            .spell_custom_attributes_for_difficulty_like_cpp(spell_id, u32::from(difficulty))
            & wow_data::SPELL_ATTR0_CU_NO_INITIAL_THREAT_LIKE_CPP
            != 0
        {
            return Some(0.0);
        }

        Some(
            hub.catalogs
                .spell_catalogs
                .spell_levels_store
                .as_deref()
                .and_then(|store| {
                    let mut difficulty_id = difficulty;
                    let mut visited = HashSet::new();
                    loop {
                        if let Some(entry) =
                            store.entry_for_spell_difficulty_like_cpp(spell_id, difficulty_id)
                        {
                            break Some(entry);
                        }
                        if difficulty_id == 0 || !visited.insert(difficulty_id) {
                            break None;
                        }
                        difficulty_id = hub
                            .catalogs
                            .difficulty_store()
                            .and_then(|difficulties| difficulties.get(u32::from(difficulty_id)))
                            .map_or(0, |difficulty| difficulty.fallback_difficulty_id);
                    }
                })
                .map_or(0.0, |entry| f32::from(entry.spell_level.max(0))),
        )
    }

    pub fn canonical_threat_aura_snapshot_for_difficulty_like_cpp(
        &self,
        hub: HubRef<'_>,
        spell_id: i32,
        difficulty: u8,
        effect_mask: u32,
        represented_effect_amounts: &[RepresentedAuraEffectAmountLikeCpp],
    ) -> CanonicalThreatAuraSnapshotLikeCpp {
        wow_world_core::session::PlayerAuraRemovalAccessLikeCpp::threat_aura_snapshot_from_stores_like_cpp(
            hub.catalogs.spell_store().map(AsRef::as_ref),
            hub.catalogs.difficulty_store().map(AsRef::as_ref),
            spell_id, difficulty, effect_mask, represented_effect_amounts,
        )
    }
}

/// Read a canonical creature's threat toward one attacker, from an already
/// locked map.
///
/// Lifted by #28: the session variant took the canonical lock itself, so a
/// caller that already held the map had to drop it and take it again.
pub fn creature_threat_value_on_map_like_cpp(
    map: &wow_map::ManagedMapInnerLikeCpp,
    creature_guid: ObjectGuid,
    attacker_guid: ObjectGuid,
) -> Option<f32> {
    map.with_creature_like_cpp(creature_guid, |creature| {
        creature
            .unit()
            .subsystems()
            .combat
            .threat_value(attacker_guid)
    })
    .flatten()
}

/// Mirror a legacy creature's threat toward its attacker into the canonical
/// map, on an already locked map.
///
/// Lifted by #28. The session variant acquired the canonical lock twice — once
/// inside `begin_canonical_player_combat_ref_like_cpp` and again for the mirror
/// itself. Taking the map as an argument collapses that to one acquisition for
/// both halves, which is also what lets the sessionless loop call it.
pub fn mirror_creature_threat_from_attacker_on_map_like_cpp(
    map: &mut wow_map::ManagedMapInnerLikeCpp,
    creature_guid: ObjectGuid,
    attacker_guid: ObjectGuid,
    threat_value: f32,
) -> bool {
    if threat_value <= 0.0 {
        return false;
    }
    if !begin_combat_ref_on_map_like_cpp(map, attacker_guid, creature_guid, false, false, false) {
        return false;
    }

    let threat_ref = {
        let Some(creature) = map.get_typed_creature_mut(creature_guid) else {
            return false;
        };
        creature
            .unit_mut()
            .subsystems_mut()
            .combat
            .set_threat(attacker_guid, threat_value);
        creature
            .unit()
            .subsystems()
            .combat
            .threat_ref(attacker_guid)
            .copied()
    };

    let Some(threat_ref) = threat_ref else {
        return false;
    };
    let Some(attacker) = map.get_typed_player_mut(attacker_guid) else {
        return false;
    };
    attacker
        .unit_mut()
        .subsystems_mut()
        .combat
        .put_threatened_by_me_ref(creature_guid, threat_ref);
    true
}
