//! Actual borrowed definition adapters; no copied catalog or learning admission.
use super::{SpellDefinitionError, SpellDefinitionSeeds, SpellDefinitionView, SpellEffectView};
use wow_entities::{SpellBookDefinition, SpellBookEffect, SpellDefinitionSource};

impl SpellDefinitionSource for SpellDefinitionSeeds {
    type Error = SpellDefinitionError;
    type Definition<'a> = SpellDefinitionView<'a>;

    fn definition(
        &self,
        spell: u32,
        difficulty: i16,
    ) -> Result<Option<Self::Definition<'_>>, Self::Error> {
        self.get(spell, difficulty)
    }
    fn owns<'a>(&self, definition: &Self::Definition<'a>) -> bool
    where
        Self: 'a,
    {
        definition.belongs_to(self)
    }
}

impl<'a> SpellBookDefinition for SpellDefinitionView<'a> {
    type Effect = SpellEffectView<'a>;
    fn spell_id(&self) -> u32 {
        self.spell_id()
    }
    fn attributes(&self) -> &[u32; 17] {
        &self.fields().attributes
    }
    fn custom_attributes(&self) -> u32 {
        self.custom_attributes()
    }
    fn stances(&self) -> u64 {
        self.fields().stances
    }
    fn caster_aura_state(&self) -> u32 {
        self.fields().caster_aura_state
    }
    fn equipped_item_class(&self) -> i32 {
        self.fields().equipped_item_class
    }
    fn family_name(&self) -> u32 {
        self.fields().spell_family_name
    }
    fn family_flags(&self) -> [u32; 4] {
        self.fields().spell_family_flags
    }
    fn effect(&self, index: usize) -> Option<Self::Effect> {
        self.effect(index)
    }
    fn effect_count(&self) -> usize {
        self.effect_count()
    }
    fn effects(&self) -> impl Iterator<Item = Self::Effect> + '_ {
        self.effects()
    }
    fn has_effect(&self, kind: u32) -> bool {
        self.has_effect(kind)
    }
}

impl SpellBookEffect for SpellEffectView<'_> {
    fn effect_kind(&self) -> u32 {
        self.values().effect
    }
    fn is_aura(&self) -> bool {
        self.is_aura()
    }
    fn is_aura_kind(&self, kind: u32) -> bool {
        self.is_aura_kind(kind)
    }
    fn misc_value_0(&self) -> i32 {
        self.values().misc_values[0]
    }
    fn class_mask(&self) -> [u32; 4] {
        self.values().class_mask
    }
}
