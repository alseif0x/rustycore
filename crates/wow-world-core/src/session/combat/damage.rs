use crate::session::state::SessionCore;
use wow_core::ObjectGuid;

pub const DAMAGE_FALL_LIKE_CPP: u8 = 2;
pub const DAMAGE_FALL_TO_VOID_LIKE_CPP: u8 = 6;

impl SessionCore {
    pub fn send_environmental_damage_log_like_cpp(
        &self,
        victim: ObjectGuid,
        damage_type: u8,
        amount: u32,
        resisted: u32,
        absorbed: u32,
    ) {
        self.send_packet(&wow_packet::packets::combat::EnvironmentalDamageLog {
            victim,
            damage_type: if damage_type == DAMAGE_FALL_TO_VOID_LIKE_CPP {
                DAMAGE_FALL_LIKE_CPP
            } else {
                damage_type
            },
            amount: amount.min(i32::MAX as u32) as i32,
            resisted: resisted.min(i32::MAX as u32) as i32,
            absorbed: absorbed.min(i32::MAX as u32) as i32,
        });
    }
}
