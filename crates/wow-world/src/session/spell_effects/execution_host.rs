//! The World session as the host of the spell execution moved to
//! `wow_world_spell` (#1363 slice 9d). Each operation names the inherent
//! `WorldSession` method by path.

use super::*;
use std::future::Future;

impl wow_world_spell::SpellExecutionHostLikeCpp for WorldSession {
    fn mutate_cast_execution_like_cpp<R>(
        &mut self,
        f: impl FnOnce(&mut wow_entities::CastExecutionStateLikeCpp) -> R,
    ) -> Option<R> {
        let (state, mut hub) = crate::session::split_spell_state_mut(self);
        wow_world_application::mutate_cast_execution_like_cpp(&mut hub, state, f)
    }
    fn has_represented_battle_pet_journal_lock_like_cpp(&self) -> bool {
        wow_world_inventory::has_represented_battle_pet_journal_lock_like_cpp(
            crate::session::hub_ref(self),
            &self.lifecycle,
            cfg!(test),
        )
    }
    fn represented_battle_pet_like_cpp(
        &self,
        pet_guid: ObjectGuid,
    ) -> Option<RepresentedBattlePetDataLikeCpp> {
        wow_world_inventory::represented_battle_pet_like_cpp(
            crate::session::hub_ref(self),
            &self.lifecycle,
            cfg!(test),
            pet_guid,
        )
    }
    fn force_update_visibility_with_catalogs_like_cpp(
        &mut self,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
    ) -> impl Future<Output = ()> + Send {
        wow_world_entities::force_update_visibility_with_catalogs_like_cpp(
            self,
            creature_spawn_catalogs,
        )
    }
    fn consume_add_farsight_set_seer_outcome_like_cpp(
        &mut self,
        outcome: &wow_map::map::FarsightDynamicObjectCreateOutcomeLikeCpp,
    ) -> bool {
        wow_world_spell::consume_add_farsight_set_seer_outcome_like_cpp(self, outcome)
    }
    fn represented_nearby_entry_destination_like_cpp(
        &self,
        effect: &wow_data::SpellEffectInfo,
        caster_position: &Position,
        range: f32,
        implicit_conditions: Option<&[wow_data::Condition]>,
    ) -> Option<Position> {
        wow_world_spell::represented_nearby_entry_destination_like_cpp(
            self,
            effect,
            caster_position,
            range,
            implicit_conditions,
        )
    }
    fn apply_creature_aura_with_provenance_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        target_guid: ObjectGuid,
        effect_mask: u32,
        duration_ms: u32,
        provenance: wow_entities::AuraCastProvenanceLikeCpp,
    ) -> Result<(), &'static str> {
        wow_world_entities::apply_creature_aura_with_provenance_like_cpp(
            self,
            spell_id,
            caster_guid,
            target_guid,
            effect_mask,
            duration_ms,
            provenance,
        )
    }
    fn apply_aura_with_effect_mask_and_provenance_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        duration_ms: u32,
        aura_flags: u32,
        effect_mask: u32,
        provenance: wow_entities::AuraCastProvenanceLikeCpp,
    ) -> Result<(), &'static str> {
        WorldSession::apply_aura_with_effect_mask_and_provenance_like_cpp(
            self,
            spell_id,
            caster_guid,
            duration_ms,
            aura_flags,
            effect_mask,
            provenance,
        )
    }
    fn apply_durability_damage_effect_like_cpp(
        &mut self,
        effect: u32,
        damage: i32,
        slot: i32,
        target_guid: ObjectGuid,
    ) {
        wow_world_spell::apply_durability_damage_effect_like_cpp(
            self,
            effect,
            damage,
            slot,
            target_guid,
        )
    }
    fn apply_durability_damage_pct_effect_like_cpp(
        &mut self,
        damage: i32,
        slot: i32,
        target_guid: ObjectGuid,
    ) {
        wow_world_inventory::apply_durability_damage_pct_effect_like_cpp(
            self,
            damage,
            slot,
            target_guid,
        )
    }
    fn apply_duel_effect_like_cpp(
        &mut self,
        spell_id: i32,
        gameobject_entry: i32,
        target_guid: ObjectGuid,
    ) -> bool {
        wow_world_social::apply_duel_effect_like_cpp(self, spell_id, gameobject_entry, target_guid)
    }
    fn apply_learn_spell_effect_like_cpp(
        &mut self,
        spell: i32,
        target: ObjectGuid,
    ) -> impl Future<Output = bool> + Send {
        WorldSession::apply_learn_spell_effect_like_cpp(self, spell, target)
    }
    fn apply_reputation_effect_like_cpp(
        &mut self,
        damage: i32,
        misc_value: i32,
        target_guid: ObjectGuid,
    ) -> bool {
        wow_world_social::apply_reputation_effect_like_cpp(self, damage, misc_value, target_guid)
    }
    fn apply_quest_complete_effect_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        target_guid: ObjectGuid,
        quest_id: i32,
    ) -> impl Future<Output = Result<(), &'static str>> + Send {
        wow_world_quest::apply_quest_complete_effect_like_cpp(
            self,
            item_guid_generator,
            target_guid,
            quest_id,
        )
    }
    fn apply_force_deselect_effect_like_cpp(&mut self) -> bool {
        wow_world_social::apply_force_deselect_effect_like_cpp(self)
    }
    fn player_resurrection_state_snapshot_like_cpp(
        &self,
    ) -> Option<PlayerResurrectionStateLikeCpp> {
        WorldSession::player_resurrection_state_snapshot_like_cpp(self)
    }
    fn set_player_ghost_flag_like_cpp(&mut self, ghost: bool) {
        WorldSession::set_player_ghost_flag_like_cpp(self, ghost)
    }
    fn set_player_alive_like_cpp(&mut self, alive: bool) {
        WorldSession::set_player_alive_like_cpp(self, alive)
    }
    #[cfg(test)]
    fn mutate_player_persistent_capability_state_like_cpp<R>(
        &mut self,
        mutate: impl FnOnce(&mut wow_entities::PlayerPersistentCapabilityStateLikeCpp) -> R,
    ) -> Option<R> {
        WorldSession::mutate_player_persistent_capability_state_like_cpp(self, mutate)
    }
    #[cfg(test)]
    fn set_represented_seer_guid_fixture_like_cpp(&mut self, guid: Option<ObjectGuid>) {
        self.visibility
            .set_represented_seer_guid_fixture_like_cpp(guid)
    }
}
