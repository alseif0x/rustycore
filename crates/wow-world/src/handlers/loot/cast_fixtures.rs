//! Loot interruption fixtures operate on the resident Player's existing cast and aura owners.
use super::*;

pub fn install_loot_interruptible_cast_for_test(session: &mut WorldSession, player: ObjectGuid) -> bool {
    session.set_active_spell_cast_like_cpp(Some(crate::session::SpellCastState {
        spell_id: 133,
        target_guid: player,
        target_data: wow_entities::SpellCastTargetsLikeCpp { flags: 0x2, unit: player, ..Default::default() },
        cast_id: ObjectGuid::create_world_object(wow_core::guid::HighGuid::Cast, 0, 1, 0, 0, 1, 7),
        cast_start_time: Instant::now(),
        cast_time_ms: 30_000,
        spell_visual: wow_entities::SpellCastVisualLikeCpp { spell_visual_id: 1, script_visual_id: 0 },
        metadata: crate::session::SpellCastMetadata::default(),
    }))
}

pub fn install_loot_interrupt_aura_for_test(session: &mut WorldSession, slot: u8, spell_id: i32, caster_guid: ObjectGuid, aura_interrupt_flags: u32) -> bool {
    session.insert_player_visible_aura_like_cpp(crate::session::AuraApplication {
        spell_id, difficulty_id: 0, caster_guid, slot,
        duration_total: 30_000, duration_remaining: 30_000, stack_count: 1,
        aura_flags: 0x0000_0001, effect_mask: 0x0000_0001,
        aura_interrupt_flags, aura_interrupt_flags2: 0,
        represented_effect: None, represented_amount: 0,
        represented_effect_amounts: Vec::new(), represented_misc_value: None,
        represented_multiplier: 1.0, applied_at: Instant::now(),
    })
}

pub fn loot_cast_pending_for_test(session: &WorldSession) -> bool {
    session.loot_cast_pending()
}

pub fn loot_aura_slot_present_for_test(session: &WorldSession, slot: u8) -> bool {
    session.loot_aura_slot_present(slot)
}
