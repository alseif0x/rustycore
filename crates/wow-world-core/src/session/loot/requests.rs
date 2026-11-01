// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

impl crate::session::SessionCatalogs {
    pub fn represented_spell_max_range_like_cpp(&self, spell_id: i32) -> Option<f32> {
        let spell_store = self.spell_store()?;
        let spell_misc_store = self.spell_catalogs.spell_misc_store()?;
        let spell_range_store = self.spell_catalogs.spell_range_store()?;
        spell_store.get(spell_id)?;
        let spell_id = u32::try_from(spell_id).ok()?;
        let range_index = spell_misc_store.get(spell_id)?.range_index;
        let range = spell_range_store.get(u32::from(range_index))?;
        Some(range.range_max[1].max(range.range_max[0]))
    }
}
