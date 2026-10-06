use super::*;
pub(in crate::session) use wow_world_spell::PlayerCastPublicationPhaseLikeCpp;

impl WorldSession {
    pub(in crate::session) fn player_cast_publication_like_cpp(
        &self,
        spell: &wow_data::SpellInfo,
        metadata: &SpellCastMetadata,
        phase: PlayerCastPublicationPhaseLikeCpp,
    ) -> (wow_packet::packets::spell::SpellCastData, u32) {
        let (state, hub) = crate::session::split_spell_state_ref(self);
        state.player_cast_publication_like_cpp(hub, spell, metadata, phase)
    }
}
