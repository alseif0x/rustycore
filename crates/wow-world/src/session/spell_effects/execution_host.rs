//! The World session as the host of spell execution moved to `wow_world_spell`.
//! Cross-domain normal Player costs are delegated to the Application owner.

use super::*;
use std::future::Future;
use std::pin::Pin;

impl wow_world_spell::SpellExecutionHostLikeCpp for WorldSession {
    fn server_normal_physical_item_facts_like_cpp(
        &self,
        item_guid: ObjectGuid,
    ) -> Option<(u32, u32)> {
        let item = self.core.with_owned_player_like_cpp(|player| {
            player
                .inventory_runtime_like_cpp()
                .item_objects()
                .get(&item_guid)
                .filter(|item| {
                    item.owner_guid() == player.guid()
                        && !item.is_in_trade()
                        && item.update_state() != wow_constants::ItemUpdateState::Removed
                })
                .cloned()
        })??;
        let entry_id = item.object().entry();
        let item_level = self.inventory.represented_item_level_with_access_like_cpp(
            &self.core.inventory_valuation_access_like_cpp(),
            &self.core.owned_item_modifiers_access_like_cpp(),
            &self.catalogs.inventory_valuation_catalog_view_like_cpp(),
            entry_id,
            Some(&item),
            Self::MIN_ITEM_LEVEL_LIKE_CPP,
            Self::MAX_ITEM_LEVEL_LIKE_CPP,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.identity.player_level,
        )?;
        Some((entry_id, item_level))
    }

    fn server_normal_pre_go_like_cpp(
        &mut self,
        spell_info: &wow_data::SpellInfo,
        cast_id: ObjectGuid,
        visual: &wow_packet::packets::spell::SpellCastVisual,
        metadata: wow_entities::SpellCastMetadata,
    ) -> Result<
        wow_world_spell::ServerNormalPreGoPlanLikeCpp,
        wow_world_spell::ServerNormalPreGoFailureLikeCpp,
    > {
        wow_world_application::server_normal_pre_go_like_cpp(
            self, spell_info, cast_id, visual, metadata,
        )
    }

    fn take_server_normal_cast_item_like_cpp(
        &mut self,
        spell_info: &wow_data::SpellInfo,
        metadata: wow_entities::SpellCastMetadata,
    ) -> Result<wow_entities::SpellCastMetadata, &'static str> {
        wow_world_application::take_server_normal_cast_item_like_cpp(self, spell_info, metadata)
    }

    fn spell_cast_hook_wanted_like_cpp(&self) -> bool {
        self.module_hooks
            .wants_player_hook_like_cpp(wow_module_api::PlayerHook::SpellCast)
    }
    fn spell_cast_hook_like_cpp(&mut self, event: &wow_world_spell::SpellCastHookEventLikeCpp) {
        self.module_hooks
            .spell_cast_like_cpp(self.core.packet_publication_access_like_cpp(), event);
    }
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
        send_update: bool,
    ) -> Result<(), &'static str> {
        self.player_aura_application_cx_like_cpp()
            .apply_aura_with_effect_mask_provenance_and_update_like_cpp(
                spell_id,
                caster_guid,
                duration_ms,
                aura_flags,
                effect_mask,
                provenance,
                send_update,
            )
    }
    fn apply_ghost_wisp_aura(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        spell_info: &wow_data::SpellInfo,
        duration_ms: u32,
        effect_mask: u32,
        stack_amount: u32,
        attributes1: u32,
        attributes5: u32,
        creation: Result<(u32, wow_entities::AuraCastProvenanceLikeCpp, u8), &'static str>,
    ) -> Result<(), &'static str> {
        self.player_aura_application_cx_like_cpp()
            .apply_ghost_wisp_aura(
                spell_id,
                caster_guid,
                spell_info,
                duration_ms,
                effect_mask,
                stack_amount,
                attributes1,
                attributes5,
                creation,
            )
    }
    fn apply_resurrection_sickness_aura(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        spell_info: &wow_data::SpellInfo,
        duration_total: i32,
        stack_amount: u32,
        attributes1: u32,
        attributes5: u32,
        creation: Result<(u32, wow_entities::AuraCastProvenanceLikeCpp, u8), &'static str>,
        cast_metadata: wow_entities::SpellCastMetadata,
    ) -> Result<(), &'static str> {
        self.player_aura_application_cx_like_cpp()
            .apply_resurrection_sickness_aura(
                spell_id,
                caster_guid,
                spell_info,
                duration_total,
                stack_amount,
                attributes1,
                attributes5,
                creation,
                cast_metadata,
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
    fn apply_stuck_effect_like_cpp<'a>(
        &'a mut self,
        item_guid_generator: &'a wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &'a CreatureSpawnCatalogsLikeCpp,
        caster_guid: ObjectGuid,
    ) -> Pin<Box<dyn Future<Output = Result<(), &'static str>> + Send + 'a>> {
        Box::pin(wow_world_application::apply_stuck_effect_like_cpp(
            self,
            item_guid_generator,
            creature_spawn_catalogs,
            caster_guid,
        ))
    }
    fn self_resurrect_player_like_cpp<'a>(
        &'a mut self,
        item_guid_generator: &'a wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &'a CreatureSpawnCatalogsLikeCpp,
        health: u32,
        mana: u32,
    ) -> Pin<Box<dyn Future<Output = Result<(), &'static str>> + Send + 'a>> {
        Box::pin(wow_world_entities::self_resurrect_player_like_cpp(
            self,
            wow_world_entities::ResurrectionOperationCatalogs {
                item_guid_generator,
                creature_spawns: creature_spawn_catalogs,
                graveyards: None,
            },
            health,
            mana,
        ))
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
