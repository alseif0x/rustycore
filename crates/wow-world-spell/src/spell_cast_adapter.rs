use wow_entities::SpellCastVisualLikeCpp;
use wow_packet::packets::spell::SpellCastVisual;

pub fn present_visual(value: SpellCastVisualLikeCpp) -> SpellCastVisual {
    SpellCastVisual {
        spell_visual_id: value.spell_visual_id,
        script_visual_id: value.script_visual_id,
    }
}
