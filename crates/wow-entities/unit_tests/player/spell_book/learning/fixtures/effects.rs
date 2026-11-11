use super::*;
use std::collections::{HashMap, HashSet};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) enum Event {
    Cast(u32),
    Aura(u32),
    Fits(u32),
    Trait(i32),
    Points(u32),
    Skill(u32, u16, u16, u16),
    Criterion(SpellLearnCriterion, u32),
    MountQuery(u32),
    Mount(u32, bool, bool),
    Quest(u32),
    Message(SpellBookMessage, bool, bool),
    Order,
    OwnedAura(u32, Option<PlayerSpellState>),
    PetQuery(u32, u8),
    PetRemove(u32, u8),
    RemoveAura(u32),
    TitanOff,
    DualOff,
    Offhand,
}
#[derive(Default)]
pub(in super::super) struct Effects<'a> {
    pub events: Vec<Event>,
    pub world: bool,
    pub loading: bool,
    pub form: u32,
    pub item_fits: bool,
    pub auras: HashSet<u32>,
    pub aura_states: HashSet<u32>,
    pub traits: HashMap<i32, u32>,
    pub free: u32,
    pub skills: HashMap<u32, (u16, u16)>,
    pub mounts: HashSet<u32>,
    pub fail: Option<Event>,
    pub bad_order: bool,
    pub cast_insert: Option<u32>,
    pub mount_reenter: bool,
    pub trait_unavailable: bool,
    pub skill_reenter: Option<(u32, u32)>,
    pub reentry_sources: Option<&'a Sources<'a>>,
    pub pet_auras: HashSet<(u32, u8)>,
    pub pet_catalog_unavailable: bool,
    pub max_professions: u32,
    pub titan: bool,
    pub titan_penalty: u32,
    pub dual: bool,
    pub offhand: bool,
    pub aura_reenter: Option<(u32, RemovePlayerSpell)>,
}
impl Effects<'_> {
    pub fn quiet() -> Self {
        Self {
            loading: true,
            item_fits: true,
            max_professions: 2,
            ..Default::default()
        }
    }
    fn record(&mut self, event: Event) -> Result<(), &'static str> {
        self.events.push(event);
        if self.fail == Some(event) {
            Err("required effect failed")
        } else {
            Ok(())
        }
    }
}
impl<'source> SpellLearningEffects<Sources<'source>> for Effects<'_> {
    type Error = &'static str;
    fn is_in_world(&self) -> bool {
        self.world
    }
    fn is_player_loading(&self) -> bool {
        self.loading
    }
    fn player_level(&self) -> u8 {
        10
    }
    fn shapeshift_form(&self) -> u32 {
        self.form
    }
    fn has_aura(&self, spell: u32) -> bool {
        self.auras.contains(&spell)
    }
    fn has_aura_state(&self, state: u32) -> bool {
        self.aura_states.contains(&state)
    }
    fn item_fits_spell<'view>(&mut self, view: &Definition<'view>) -> Result<bool, Self::Error>
    where
        Sources<'source>: 'view,
    {
        self.record(Event::Fits(view.spell_id()))?;

        Ok(self.item_fits)
    }
    fn add_aura(&mut self, _: &mut PlayerSpellBook, spell: u32) -> Result<(), Self::Error> {
        self.record(Event::Aura(spell))
    }
    fn cast_triggered(
        &mut self,
        book: &mut PlayerSpellBook,
        spell: u32,
    ) -> Result<(), Self::Error> {
        self.record(Event::Cast(spell))?;
        if let Some(id) = self.cast_insert.take() {
            book.add_temporary_spell(id);
        }
        Ok(())
    }
    fn trait_override(&mut self, id: i32) -> Result<Option<u32>, Self::Error> {
        self.record(Event::Trait(id))?;
        if self.trait_unavailable {
            return Err("trait catalog unavailable");
        }
        Ok(self.traits.get(&id).copied())
    }
    fn free_profession_points(&self) -> u32 {
        self.free
    }
    fn set_free_profession_points(&mut self, points: u32) -> Result<(), Self::Error> {
        self.record(Event::Points(points))?;
        self.free = points;
        Ok(())
    }
    fn pure_skill_value(&self, skill: u32) -> u16 {
        self.skills.get(&skill).map_or(0, |values| values.0)
    }
    fn pure_skill_maximum(&self, skill: u32) -> u16 {
        self.skills.get(&skill).map_or(0, |values| values.1)
    }
    fn has_skill(&self, skill: u32) -> bool {
        self.pure_skill_value(skill) != 0
    }
    fn set_skill(
        &mut self,
        book: &mut PlayerSpellBook,
        input: SkillUpdate,
    ) -> Result<(), Self::Error> {
        // Command observation only, NOT actual SetSkill/Unit or durability proof.
        self.record(Event::Skill(
            input.skill,
            input.step,
            input.value,
            input.maximum,
        ))?;
        self.skills
            .insert(input.skill, (input.value, input.maximum));
        if self
            .skill_reenter
            .is_some_and(|(skill, _)| input.skill == skill)
        {
            let (_, spell) = self.skill_reenter.take().unwrap();
            let sources = self.reentry_sources.expect("scoped test reentry sources");
            book.add_spell(AddPlayerSpell::learned(spell, true, true), sources, self)
                .map_err(|_| "reentrant learning failed")?;
        }
        Ok(())
    }
    fn update_criterion(
        &mut self,
        _: &mut PlayerSpellBook,
        kind: SpellLearnCriterion,
        id: u32,
    ) -> Result<(), Self::Error> {
        self.record(Event::Criterion(kind, id))
    }
    fn has_mount_definition(&mut self, spell: u32) -> Result<bool, Self::Error> {
        self.record(Event::MountQuery(spell))?;
        Ok(self.mounts.contains(&spell))
    }
    fn add_mount(
        &mut self,
        book: &mut PlayerSpellBook,
        spell: u32,
        loading: bool,
    ) -> Result<(), Self::Error> {
        self.record(Event::Mount(spell, loading, book.has_spell(spell)))?;
        if self.mount_reenter {
            book.add_temporary_spell(spell);
        } // Same book: must not overwrite admitted node.
        Ok(())
    }
    fn quest_learn_spell(
        &mut self,
        _: &mut PlayerSpellBook,
        spell: u32,
    ) -> Result<(), Self::Error> {
        self.record(Event::Quest(spell))
    }
    fn book_order(
        &mut self,
        history: &[SpellBookMutation],
        _: usize,
    ) -> Result<Vec<u32>, Self::Error> {
        self.record(Event::Order)?;
        if self.bad_order {
            return Ok(vec![]);
        }
        let mut keys = vec![];
        for event in history {
            match *event {
                SpellBookMutation::Insert(id) => keys.push(id),
                SpellBookMutation::Erase(id) => keys.retain(|key| *key != id),
            }
        }
        keys.reverse();
        Ok(keys) // Synthetic order only, NOT pinned GNU proof.
    }
    fn publish(
        &mut self,
        book: &PlayerSpellBook,
        message: SpellBookMessage,
    ) -> Result<(), Self::Error> {
        let (old, new) = match message {
            SpellBookMessage::Superseded { old, new } => (old, new),
            SpellBookMessage::Learned { spell, .. } | SpellBookMessage::Unlearned { spell, .. } => {
                (spell, spell)
            }
        };
        self.record(Event::Message(
            message,
            book.has_active_spell(old),
            book.has_active_spell(new),
        ))
    }
    fn remove_owned_aura(
        &mut self,
        book: &mut PlayerSpellBook,
        spell: u32,
    ) -> Result<(), Self::Error> {
        self.record(Event::OwnedAura(
            spell,
            book.spell(spell).map(|entry| entry.state()),
        ))?;
        if self
            .aura_reenter
            .is_some_and(|(trigger, _)| trigger == spell)
        {
            let (_, request) = self.aura_reenter.take().unwrap();
            let sources = self.reentry_sources.expect("scoped test reentry sources");
            book.remove_spell(request, sources, self)
                .map_err(|_| "reentrant removal failed")?;
        }
        Ok(())
    }
    fn remove_pet_aura_if_defined(
        &mut self,
        _: &mut PlayerSpellBook,
        spell: u32,
        slot: u8,
    ) -> Result<(), Self::Error> {
        self.record(Event::PetQuery(spell, slot))?;
        if self.pet_catalog_unavailable {
            return Err("pet aura catalog unavailable");
        }
        if self.pet_auras.contains(&(spell, slot)) {
            self.record(Event::PetRemove(spell, slot))?;
        }
        Ok(())
    }
    fn max_primary_professions(&self) -> u32 {
        self.max_professions
    }
    fn can_titan_grip(&self) -> bool {
        self.titan
    }
    fn titan_grip_penalty_spell(&self) -> u32 {
        self.titan_penalty
    }
    fn remove_auras_due_to_spell(
        &mut self,
        _: &mut PlayerSpellBook,
        spell: u32,
    ) -> Result<(), Self::Error> {
        self.record(Event::RemoveAura(spell))
    }
    fn disable_titan_grip(&mut self) -> Result<(), Self::Error> {
        self.record(Event::TitanOff)?;
        self.titan = false;
        self.titan_penalty = 0;
        Ok(())
    }
    fn can_dual_wield(&self) -> bool {
        self.dual
    }
    fn disable_dual_wield(&mut self) -> Result<(), Self::Error> {
        self.record(Event::DualOff)?;
        self.dual = false;
        Ok(())
    }
    fn offhand_check_at_unlearn(&self) -> bool {
        self.offhand
    }
    fn auto_unequip_offhand(&mut self, _: &mut PlayerSpellBook) -> Result<(), Self::Error> {
        self.record(Event::Offhand)
    }
}
