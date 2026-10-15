//! The represented reputation effect, which changes progression state.
//!
//! Moved out of the Session root under #621. Behaviour is preserved. The raid
//! marker effect moved to `wow_world_spell` (#1363 slice 9d); the proficiency
//! effect moved there and the quest-complete effect to `wow_world_quest`
//! (#1363 slice 12b2).

use super::*;

impl WorldSession {
    /// C++ `Spell::EffectReputation`.
    ///
    /// Represented boundary: current player target only. This follows the C++
    /// spell-source reputation formula and `ReputationMgr::ModifyReputation`
    /// packet path; generic player targets, persistence and script callbacks
    /// remain outside this bounded slice.
    pub(in crate::session) fn apply_reputation_effect_like_cpp(
        &mut self,
        damage: i32,
        misc_value: i32,
        target_guid: ObjectGuid,
    ) -> bool {
        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        if target_guid != player_guid {
            return false;
        }
        let Ok(faction_id) = u32::try_from(misc_value) else {
            return false;
        };
        let Some(faction_store) = self.catalogs.faction_store().map(Arc::clone) else {
            return false;
        };
        let Some(faction_entry) = faction_store.get(faction_id).cloned() else {
            return false;
        };

        let reputation = wow_world_social::calculate_reputation_gain_like_cpp(
            self,
            ReputationGainSourceLikeCpp::Spell,
            0,
            damage,
            faction_id,
            false,
        );

        let reputation_spillover_template_store = self
            .catalogs
            .reputation_spillover_template_store()
            .map(Arc::clone);
        let friendship_rep_reaction_store = self
            .catalogs
            .friendship_rep_reaction_store()
            .map(Arc::clone);
        let paragon_reputation_store = self.catalogs.paragon_reputation_store().map(Arc::clone);
        let currency_types_store = self.currency_types_store().map(Arc::clone);
        let db_spillover_template = reputation_spillover_template_store
            .as_deref()
            .and_then(|store| store.get(faction_id));
        let options = wow_progression::mgr::SetReputationOptionsLikeCpp {
            incremental: true,
            spillover_only: false,
            no_spillover: false,
            reputation_gain_rate: self.config.reputation_rates_like_cpp().gain,
            paragon_reward_quest_status_none_like_cpp: true,
            renown_current_level_like_cpp: 0,
            renown_currency_increased_cap_quantity_like_cpp: 0,
            player_race: crate::session::hub_ref(self).player_race_like_cpp(),
            player_class: crate::session::hub_ref(self).player_class_like_cpp(),
        };
        let Some((outcome, packet)) =
            crate::session::hub_mut(self).mutate_reputation_mgr_like_cpp(|mgr| {
                let outcome = mgr.set_reputation_like_cpp(
                    &faction_entry,
                    reputation,
                    options,
                    &faction_store,
                    db_spillover_template,
                    friendship_rep_reaction_store.as_deref(),
                    paragon_reputation_store.as_deref(),
                    currency_types_store.as_deref(),
                );
                let packet = outcome
                    .send_state_rep_list_id
                    .map(|rep_list_id| mgr.set_faction_standing_packet_like_cpp(Some(rep_list_id)));
                (outcome, packet)
            })
        else {
            return false;
        };
        if let Some(packet) = packet {
            self.send_packet(&packet);
        }
        outcome.applied
    }
}
