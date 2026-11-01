impl crate::SessionSpellState {
    pub fn restore_represented_character_spell_charge_like_cpp(
        &mut self,
        hub: &mut wow_world_core::session::HubMut<'_>,
        category_id: u32,
    ) -> bool {
        self.mutate_player_spell_history_like_cpp(hub, |history| {
            if !history.charges_loaded {
                return false;
            }
            let Some(charges) = history.charges.get_mut(&category_id) else {
                return false;
            };
            if charges.pop_back().is_none() {
                return false;
            }
            if charges.is_empty() {
                history.charges.remove(&category_id);
            }
            true
        })
        .unwrap_or(false)
    }
}
