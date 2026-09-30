//! Borrowed observations of the actual gameobject callback rail.
use super::*;

#[derive(Debug)]
pub struct LootUseEffect(RepresentedGameObjectUseEffect);
pub struct LootUseEffects<'a>(&'a [RepresentedGameObjectUseEffect]);

impl std::fmt::Debug for LootUseEffects<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}
impl PartialEq<Vec<LootUseEffect>> for LootUseEffects<'_> {
    fn eq(&self, expected: &Vec<LootUseEffect>) -> bool {
        self.0.len() == expected.len() && self.0.iter().zip(expected).all(|(actual, expected)| actual == &expected.0)
    }
}
impl LootUseEffect {
    pub fn trigger_event(gameobject_guid: ObjectGuid, player_guid: ObjectGuid, event_id: u32) -> Self {
        Self(RepresentedGameObjectUseEffect::TriggerGameEvent { gameobject_guid, player_guid, event_id })
    }
    pub fn trigger_trap(gameobject_guid: ObjectGuid, player_guid: ObjectGuid, trap_entry: u32) -> Self {
        Self(RepresentedGameObjectUseEffect::TriggerLinkedTrap { gameobject_guid, player_guid, trap_entry })
    }
    pub fn fishing_catch(gameobject_guid: ObjectGuid, player_guid: ObjectGuid, gameobject_entry: u32) -> Self {
        Self(RepresentedGameObjectUseEffect::FishingHoleCatchCriteriaUpdated { gameobject_guid, player_guid, gameobject_entry })
    }
    pub fn outdoor_spell(gameobject_guid: ObjectGuid, player_guid: ObjectGuid, gameobject_entry: u32, spell_id: u32, go_type: u32, spell_lookup_difficulty_id: u8, spell_info_missing: bool) -> Self {
        Self(RepresentedGameObjectUseEffect::OutdoorPvpCustomSpellRequested { gameobject_guid, player_guid, gameobject_entry, spell_id, go_type, spell_lookup_difficulty_id, spell_info_missing })
    }
    pub fn post_use_spell(gameobject_guid: ObjectGuid, target_guid: ObjectGuid, caster_guid: ObjectGuid, spell_id: u32, triggered: bool, spell_lookup_difficulty_id: u8) -> Self {
        Self(RepresentedGameObjectUseEffect::GameObjectPostUseSpellCast { gameobject_guid, target_guid, caster_guid, spell_id, triggered, caster: RepresentedGameObjectSpellCaster::User, spell_lookup_difficulty_id })
    }
}
pub fn gameobject_loot_effects_for_test(session: &WorldSession) -> LootUseEffects<'_> {
    LootUseEffects(&session.represented_gameobject_use_effects)
}
