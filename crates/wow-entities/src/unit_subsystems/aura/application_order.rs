use super::AuraSubsystem;

impl AuraSubsystem {
    /// Return selected runtime aura slots in canonical application order.
    /// Every matching runtime application must have an applied-aura reference,
    /// or removal order cannot be proven and the caller must preserve state.
    pub fn ordered_slots_for_spell_removal(&self, spell_id: i32) -> Result<Vec<u8>, &'static str> {
        let spell_id_u32 =
            u32::try_from(spell_id).map_err(|_| "Invalid spell id for canonical aura removal")?;
        let mut slots = Vec::new();
        for applied in &self.applied_auras {
            if applied.spell_id != spell_id_u32 {
                continue;
            }
            let application = self
                .runtime_application_like_cpp(applied.slot)
                .ok_or("Applied aura has no canonical runtime application")?;
            if application.spell_id != spell_id
                || application.caster_guid != applied.caster_guid
                || application.slot != applied.slot
            {
                return Err("Applied aura key differs from its canonical application");
            }
            if !slots.contains(&applied.slot) {
                slots.push(applied.slot);
            }
        }
        if self
            .runtime_applications_like_cpp()
            .iter()
            .any(|(&slot, aura)| aura.spell_id == spell_id && !slots.contains(&slot))
        {
            return Err("Canonical selected aura is missing its applied-aura key");
        }
        Ok(slots)
    }
}
