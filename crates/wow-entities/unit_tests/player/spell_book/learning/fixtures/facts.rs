use super::*;

impl SpellBookEffect for Effect<'_> {
    fn effect_kind(&self) -> u32 {
        self.0.kind
    }
    fn is_aura(&self) -> bool {
        self.0.aura
    }
    fn is_aura_kind(&self, kind: u32) -> bool {
        self.0
            .aura_kinds
            .iter()
            .find(|(key, _)| *key == kind)
            .expect("explicit aura-kind answer")
            .1
    }
    fn misc_value_0(&self) -> i32 {
        self.0.misc
    }
    fn class_mask(&self) -> [u32; 4] {
        self.0.mask
    }
}
impl<'a> SpellBookDefinition for Definition<'a> {
    type Effect = Effect<'a>;
    fn spell_id(&self) -> u32 {
        self.0.id
    }
    fn attributes(&self) -> &[u32; 17] {
        &self.0.attributes
    }
    fn custom_attributes(&self) -> u32 {
        self.0.custom
    }
    fn stances(&self) -> u64 {
        self.0.stances
    }
    fn caster_aura_state(&self) -> u32 {
        self.0.caster_state
    }
    fn equipped_item_class(&self) -> i32 {
        self.0.equipped_class
    }
    fn family_name(&self) -> u32 {
        self.0.family
    }
    fn family_flags(&self) -> [u32; 4] {
        self.0.family_flags
    }
    fn effect(&self, index: usize) -> Option<Self::Effect> {
        self.0.effects.get(index).map(Effect)
    }
    fn effect_count(&self) -> usize {
        self.0.effects.len()
    }
    fn effects(&self) -> impl Iterator<Item = Self::Effect> + '_ {
        self.0.effects.iter().map(Effect)
    }
    fn has_effect(&self, kind: u32) -> bool {
        self.0
            .has_effect
            .iter()
            .find(|(key, _)| *key == kind)
            .expect("explicit effect-presence answer")
            .1
    }
}
impl SpellDefinitionSource for Sources<'_> {
    type Error = Infallible;
    type Definition<'a>
        = Definition<'a>
    where
        Self: 'a;
    fn definition(
        &self,
        spell: u32,
        difficulty: i16,
    ) -> Result<Option<Self::Definition<'_>>, Self::Error> {
        assert_eq!(
            difficulty, 0,
            "only regular lookup is admitted in these cases"
        );
        Ok(self.spell(spell).definition.as_ref().map(Definition))
    }
    fn owns<'a>(&self, view: &Self::Definition<'a>) -> bool
    where
        Self: 'a,
    {
        self.0.spells.iter().any(|row| {
            row.definition
                .as_ref()
                .is_some_and(|facts| std::ptr::eq(facts, view.0))
        })
    }
}
impl SpellLearningSource for Sources<'_> {
    type WorldError = Infallible;
    type ValidityError = Infallible;
    fn spell_is_valid(&self, spell: u32) -> Result<bool, Self::ValidityError> {
        Ok(self.spell(spell).valid)
    }
    fn spell_rank_node(&self, spell: u32) -> Option<SpellLearningRank> {
        self.spell(spell).rank
    }
    fn previous_spell_in_chain(&self, spell: u32) -> u32 {
        self.spell(spell).previous
    }
    fn next_spell_in_chain(&self, spell: u32) -> u32 {
        self.spell(spell).next
    }
    fn first_spell_in_chain(&self, spell: u32) -> u32 {
        self.spell(spell).first
    }
    fn spells_requiring(&self, spell: u32) -> Option<impl Iterator<Item = u32> + '_> {
        Some(self.spell(spell).required.iter().copied())
    }
    fn spell_learn_nodes(
        &self,
        spell: u32,
    ) -> Option<impl Iterator<Item = SpellLearningNode> + '_> {
        Some(self.spell(spell).forward.iter().copied())
    }
    fn spell_learned_by(&self, spell: u32) -> Option<impl Iterator<Item = SpellLearningNode> + '_> {
        Some(self.spell(spell).reverse.iter().copied())
    }
    fn skill_line_abilities(
        &self,
        spell: u32,
    ) -> Option<impl Iterator<Item = SpellLearningAbility> + '_> {
        Some(self.spell(spell).abilities.iter().copied())
    }
    fn spell_learn_skill(&self, spell: u32) -> Option<SpellLearningSkill> {
        self.spell(spell).learned_skill
    }
    fn skill_line(&self, skill: u32) -> Option<SkillLineMetadata> {
        self.0
            .lines
            .iter()
            .find(|(key, _)| *key == skill)
            .expect("explicit skill-line answer")
            .1
    }
    fn skill_race_class_info(
        &self,
        skill: u32,
    ) -> Result<Option<SkillRaceClassMetadata>, Self::WorldError> {
        Ok(self
            .0
            .race_class
            .iter()
            .find(|(key, _)| *key == skill)
            .expect("explicit RC answer")
            .1)
    }
    fn skill_tier_value(&self, tier: u32, index: u32) -> Option<u32> {
        self.0
            .tiers
            .iter()
            .find(|(key, _)| *key == (tier, index))
            .expect("explicit tier/index answer")
            .1
    }
    fn default_skill_request(&self, rc: SkillRaceClassMetadata, level: u8) -> Option<SkillUpdate> {
        self.0
            .defaults
            .iter()
            .find(|(key, _)| *key == (rc.id, level))
            .expect("explicit live-level default answer")
            .1
    }
}
