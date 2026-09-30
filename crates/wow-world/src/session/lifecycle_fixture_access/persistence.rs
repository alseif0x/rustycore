//! Feature-only Character persistence operation forwards.

use super::*;

impl WorldSession {
    pub fn character_durable_loot_money_persistence_tracker_for_test(
        &self,
    ) -> Arc<DurableLootMoneyPersistenceTrackerLikeCpp> {
        self.durable_loot_money_persistence_tracker_like_cpp()
    }

    pub fn character_load_tutorials_data_values_for_test(
        &mut self,
        values: Option<[u32; 8]>,
    ) {
        self.load_tutorials_data_values_like_cpp(values)
    }

    pub async fn character_mark_character_account_offline_for_test(
        &mut self,
    ) -> FinalizationOutcome {
        self.mark_character_account_offline_like_cpp().await
    }

    pub async fn character_mark_character_offline_for_test(
        &mut self,
    ) -> FinalizationOutcome {
        self.mark_character_offline().await
    }

    pub async fn character_mark_login_account_offline_on_disconnect_for_test(
        &mut self,
    ) -> FinalizationOutcome {
        self.mark_login_account_offline_on_disconnect_like_cpp().await
    }

    pub fn character_mark_represented_character_spell_charges_loaded_for_test(
        &mut self,
    ) {
        self.mark_represented_character_spell_charges_loaded_like_cpp()
    }

    pub fn character_mark_represented_character_spell_cooldowns_loaded_for_test(
        &mut self,
    ) {
        self.mark_represented_character_spell_cooldowns_loaded_like_cpp()
    }

    pub fn character_record_loaded_character_spell_charge_for_test(
        &mut self,
        category_id: u32,
        recharge_start_unix_secs: i64,
        recharge_end_unix_secs: i64,
    ) {
        self.record_loaded_character_spell_charge_like_cpp(category_id, recharge_start_unix_secs, recharge_end_unix_secs)
    }

    pub fn character_record_loaded_character_spell_cooldown_for_test(
        &mut self,
        spell_id: u32,
        item_id: u32,
        cooldown_end_unix_secs: i64,
        category_id: u32,
        category_end_unix_secs: i64,
    ) {
        self.record_loaded_character_spell_cooldown_like_cpp(spell_id, item_id, cooldown_end_unix_secs, category_id, category_end_unix_secs)
    }

    pub async fn character_save_current_player_to_db_for_test(
        &mut self,
    ) -> PlayerSaveOutcomeLikeCpp {
        let generators = self.id_generators_for_test_like_cpp();
        self.save_current_player_to_db_with_generator_like_cpp(generators.item.as_ref())
            .await
    }

    pub fn character_tutorials_changed_for_test(&self) -> bool {
        self.lifecycle.tutorials_changed_like_cpp
    }

    pub fn character_tutorials_loaded_for_test(&self) -> bool {
        self.lifecycle.tutorials_loaded_from_db_like_cpp
    }

    pub fn character_set_tutorials_dirty_for_test(&mut self, changed: bool, coherent: bool) {
        self.lifecycle.tutorials_changed_like_cpp = changed;
        self.lifecycle.tutorials_loaded_coherently_like_cpp = coherent;
    }
}
