use crate::session::state::SessionCore;

impl SessionCore {
    pub fn invalidate_canonical_player_spell_hit_aura_authority_like_cpp(&mut self) {
        let _ = self.mutate_canonical_player_like_cpp(|player| {
            player
                .unit_mut()
                .subsystems_mut()
                .auras
                .invalidate_spell_hit_aura_authority_like_cpp();
        });
    }
}
