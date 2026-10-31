//! Spell-state adapters for represented Player cast behavior.

use crate::SessionSpellState;
use wow_constants::PowerType;
use wow_entities::VISIBILITY_RADIUS_LIKE_CPP as VISIBILITY_RADIUS;
use wow_world_core::session::RepresentedTalentRespecVisualSpellCastLikeCpp;
use wow_world_core::session::{HubMut, HubRef, creature_ai_spell_difficulty_chain_like_cpp};

impl SessionSpellState {
    pub fn record_cast_character_spell_cooldown_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        spell_id: i32,
        actual_spell_difficulty_id: Option<u8>,
        cooldown_ms: u32,
    ) {
        if !self
            .player_spell_history_snapshot_like_cpp(hub.shared())
            .is_some_and(|history| history.cooldowns_loaded)
            || cooldown_ms == 0
        {
            return;
        }
        let Ok(spell_id) = u32::try_from(spell_id) else {
            return;
        };
        let cooldown_secs = i64::from(cooldown_ms.saturating_add(999) / 1_000);
        if cooldown_secs == 0 {
            return;
        }
        let Some(actual_spell_difficulty_id) = actual_spell_difficulty_id else {
            return;
        };
        let Some(spell_store) = hub.catalogs.spell_catalogs.spell_store.as_deref() else {
            return;
        };
        if !spell_store.contains_spell_info_exact_like_cpp(spell_id, actual_spell_difficulty_id) {
            return;
        }
        let difficulty_store = hub.catalogs.difficulty_store().map(AsRef::as_ref);
        let Ok((_, _, category_cooldown_ms)) = spell_store
            .cooldown_components_at_difficulty_like_cpp(
                spell_id as i32,
                actual_spell_difficulty_id,
                difficulty_store,
            )
        else {
            return;
        };
        let Ok(category_id) = spell_store.category_id_at_difficulty(
            spell_id,
            actual_spell_difficulty_id,
            difficulty_store,
        ) else {
            return;
        };
        let category_cooldown_secs = i64::from(category_cooldown_ms.saturating_add(999) / 1_000);
        let now = wow_world_core::session::connection_identity::unix_now();
        self.record_loaded_character_spell_cooldown_like_cpp(
            hub,
            spell_id,
            0,
            now.saturating_add(cooldown_secs),
            category_id,
            now.saturating_add(category_cooldown_secs),
        );
    }

    pub fn broadcast_to_movement_set_realm_like_cpp(
        &self,
        hub: HubRef<'_>,
        bytes: Vec<u8>,
        _include_self: bool,
    ) {
        hub.broadcast_to_movement_set_in_range_and_connection_like_cpp(
            bytes,
            VISIBILITY_RADIUS,
            true,
        );
    }

    pub fn represented_login_passive_spell_cast_gate_like_cpp(
        &self,
        hub: HubRef<'_>,
        spell_id: i32,
    ) -> bool {
        let Some(spell_store) = hub.catalogs.spell_catalogs.spell_store.as_ref() else {
            return false;
        };
        let (stances, _) = spell_store.shapeshift_masks_like_cpp(spell_id);
        let Some(form) = hub.represented_shapeshift_form_like_cpp() else {
            return false;
        };
        let stance_mask = form
            .checked_sub(1)
            .and_then(|shift| 1u64.checked_shl(shift))
            .unwrap_or(0);
        let need_cast = stances == 0
            || (form != 0 && (stances & stance_mask) != 0)
            || (form == 0
                && spell_store.has_attribute2_like_cpp(
                    spell_id,
                    wow_data::spell::attributes::SPELL_ATTR2_ALLOW_WHILE_NOT_SHAPESHIFTED_CASTER_FORM,
                ));

        if !need_cast {
            return false;
        }

        let Ok(spell_id_u32) = u32::try_from(spell_id) else {
            return false;
        };
        let caster_aura_state = hub
            .catalogs
            .spell_catalogs
            .spell_aura_restrictions_store
            .as_ref()
            .and_then(|store| {
                store
                    .entries_for_spell_id_like_cpp(spell_id_u32)
                    .find(|entry| entry.difficulty_id == 0 || entry.difficulty_id == u8::MAX)
                    .or_else(|| store.entries_for_spell_id_like_cpp(spell_id_u32).next())
            })
            .map(|entry| entry.caster_aura_state)
            .unwrap_or(0);

        caster_aura_state == 0
            || self.represented_has_aura_state_like_cpp(hub, u32::from(caster_aura_state))
    }

    #[cfg_attr(not(any(test, feature = "test-fixtures")), allow(unused_variables))]
    pub fn record_represented_talent_respec_visual_spell_cast_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        cast: RepresentedTalentRespecVisualSpellCastLikeCpp,
    ) {
        #[cfg(any(test, feature = "test-fixtures"))]
        hub.fixtures
            .progression
            .represented_talent_respec_visual_spell_casts_like_cpp
            .push(cast);
    }
}
