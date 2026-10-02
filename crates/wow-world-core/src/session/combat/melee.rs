use crate::session::state::SessionCore;

impl SessionCore {
    pub fn set_player_attack_swing_error_like_cpp(&mut self, error: Option<u8>) {
        use wow_packet::ServerPacket;
        use wow_packet::packets::combat::AttackSwingError;

        let Some(publish) = self.mutate_canonical_player_like_cpp(|player| {
            player.set_attack_swing_error_like_cpp(error)
        }) else {
            return;
        };
        if publish {
            if let Some(reason) = error {
                let _ = self.send_tx().send(AttackSwingError { reason }.to_bytes());
            }
        }
    }
}
