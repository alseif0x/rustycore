//! 02245dcd Player.cpp:2761-3118,3161-3225. Same canonical book throughout.
//! No default/production effect executor: creation is still disabled.
mod add;
mod existing;
mod learn;
mod passive;
mod ranks;
mod remove;
mod remove_skills;
mod skills;
mod sources;
#[cfg(test)]
mod tests;
use super::{PlayerSpellBook, PlayerSpellTrait, SpellBookMutation};
use crate::forever::{
    creation::SourceError,
    player::SkillUpdate,
    spells::{SpellDefinitionError, SpellDefinitionView, SpellValidityError},
};
pub use sources::SpellLearningSources;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpellLearningSourceError {
    World(SourceError),
    MissingLearningPhases,
    MismatchedBirthCatalog,
    Definition(SpellDefinitionError),
    Validity(SpellValidityError),
    InvalidBookOrder,
    UndefinedShapeshiftMask,
    UnauthorizedGlobalCleanup,
    MissingAssertedDefinition,
    RecursiveRemovalCycle,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SpellLearningError<E> {
    Source(SpellLearningSourceError),
    Effect(E),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AddPlayerSpell {
    pub spell: u32,
    pub active: bool,
    pub learning: bool,
    pub dependent: bool,
    pub disabled: bool,
    pub loading: bool,
    pub from_skill: i32,
    pub favorite: bool,
    pub trait_data: Option<PlayerSpellTrait>,
}
impl AddPlayerSpell {
    pub fn learned(spell: u32, active: bool, dependent: bool) -> Self {
        Self {
            spell,
            active,
            learning: true,
            dependent,
            disabled: false,
            loading: false,
            from_skill: 0,
            favorite: false,
            trait_data: None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LearnPlayerSpell {
    pub spell: u32,
    pub dependent: bool,
    pub from_skill: i32,
    pub suppress_messaging: bool,
    pub trait_data: Option<PlayerSpellTrait>,
}
impl LearnPlayerSpell {
    pub fn new(spell: u32, dependent: bool, from_skill: i32) -> Self {
        Self {
            spell,
            dependent,
            from_skill,
            suppress_messaging: false,
            trait_data: None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RemovePlayerSpell {
    pub spell: u32,
    pub disabled: bool,
    pub learn_low_rank: bool,
    pub suppress_messaging: bool,
}
impl RemovePlayerSpell {
    pub fn new(spell: u32) -> Self {
        Self {
            spell,
            disabled: false,
            learn_low_rank: true,
            suppress_messaging: false,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpellLearnCriterion {
    TradeskillSkillLine,
    SpellFromSkillLine,
    LearnOrKnowSpell,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpellBookMessage {
    Superseded {
        old: u32,
        new: u32,
    },
    Unlearned {
        spell: u32,
        suppress_messaging: bool,
    },
    Learned {
        spell: u32,
        favorite: bool,
        trait_definition: Option<i32>,
        suppress_messaging: bool,
    },
}

/// The full learning operation's required synchronous capabilities. Actual
/// Unit/skills/criteria/collections own their state; no book/Player copy may be
/// retained. Cast, aura, SetSkill and mount callbacks receive the same book for
/// source reentry, after all entry borrows have ended. Effects/errors retain
/// the completed prefix: discard an unadmitted construction, never save it.
pub trait SpellLearningEffects {
    type Error;
    fn is_in_world(&self) -> bool;
    fn is_player_loading(&self) -> bool;
    fn player_level(&self) -> u8;
    fn shapeshift_form(&self) -> u32;
    fn has_aura(&self, spell: u32) -> bool;
    fn has_aura_state(&self, state: u32) -> bool;
    fn item_fits_spell(&mut self, spell: &SpellDefinitionView<'_>) -> Result<bool, Self::Error>;
    fn add_aura(&mut self, book: &mut PlayerSpellBook, spell: u32) -> Result<(), Self::Error>;
    fn cast_triggered(&mut self, book: &mut PlayerSpellBook, spell: u32)
    -> Result<(), Self::Error>;
    /// None means missing in the actual admitted catalog, not unavailable data.
    fn trait_override(&mut self, definition: i32) -> Result<Option<u32>, Self::Error>;
    fn free_profession_points(&self) -> u32;
    fn set_free_profession_points(&mut self, points: u32) -> Result<(), Self::Error>;
    /// Read the same canonical PlayerSkills, never a startup request/snapshot.
    fn pure_skill_value(&self, skill: u32) -> u16;
    fn pure_skill_maximum(&self, skill: u32) -> u16;
    fn has_skill(&self, skill: u32) -> bool;
    /// Execute actual PlayerSkills::set_skill with its complete required effects
    /// on this same Player. Ordinary void/capacity returns are not failures.
    fn set_skill(
        &mut self,
        book: &mut PlayerSpellBook,
        input: SkillUpdate,
    ) -> Result<(), Self::Error>;
    fn update_criterion(
        &mut self,
        book: &mut PlayerSpellBook,
        kind: SpellLearnCriterion,
        id: u32,
    ) -> Result<(), Self::Error>;
    fn has_mount_definition(&mut self, spell: u32) -> Result<bool, Self::Error>;
    /// Source AddMount(status=NONE, favorite=false, loading=!IsInWorld()).
    fn add_mount(
        &mut self,
        book: &mut PlayerSpellBook,
        spell: u32,
        loading: bool,
    ) -> Result<(), Self::Error>;
    fn quest_learn_spell(
        &mut self,
        book: &mut PlayerSpellBook,
        spell: u32,
    ) -> Result<(), Self::Error>;
    fn book_order(
        &mut self,
        history: &[SpellBookMutation],
        count: usize,
    ) -> Result<Vec<u32>, Self::Error>;
    /// Source immediate packet publication, not a deferred success queue. The
    /// adapter must admit effective OnPacketSend hooks as non-book-mutating
    /// before enabling this contract. Arbitrary reentrant hook behavior is NOT
    /// implemented by a precomputed key list; no production adapter exists yet.
    fn publish(
        &mut self,
        book: &PlayerSpellBook,
        message: SpellBookMessage,
    ) -> Result<(), Self::Error>;
    /// RemoveOwnedAura(spell, this Player's GUID), including synchronous reentry.
    fn remove_owned_aura(
        &mut self,
        book: &mut PlayerSpellBook,
        spell: u32,
    ) -> Result<(), Self::Error>;
    /// One exact GetPetAura(spell,physical_slot), then RemovePetAura if present:
    /// erase canonical pet-aura membership and remove the current pet's entry-
    /// selected aura. Unavailable catalog is an error, not invented absence.
    fn remove_pet_aura_if_defined(
        &mut self,
        book: &mut PlayerSpellBook,
        spell: u32,
        slot: u8,
    ) -> Result<(), Self::Error>;
    fn max_primary_professions(&self) -> u32;
    fn can_titan_grip(&self) -> bool;
    fn titan_grip_penalty_spell(&self) -> u32;
    fn remove_auras_due_to_spell(
        &mut self,
        book: &mut PlayerSpellBook,
        spell: u32,
    ) -> Result<(), Self::Error>;
    /// Source SetCanTitanGrip(false): reset both subclass masks and penalty ID.
    fn disable_titan_grip(&mut self) -> Result<(), Self::Error>;
    fn can_dual_wield(&self) -> bool;
    fn disable_dual_wield(&mut self) -> Result<(), Self::Error>;
    fn offhand_check_at_unlearn(&self) -> bool;
    /// Complete AutoUnequipOffhandIfNeed(force=false), including real inventory
    /// or mail/durability handling when bags are full. Never fake a free slot,
    /// discard the item, or equate source async COMMIT submission with an ACK.
    fn auto_unequip_offhand(&mut self, book: &mut PlayerSpellBook) -> Result<(), Self::Error>;
}

fn effect<E>(result: Result<(), E>) -> Result<(), SpellLearningError<E>> {
    result.map_err(SpellLearningError::Effect)
}
fn source<E>(error: SpellLearningSourceError) -> SpellLearningError<E> {
    SpellLearningError::Source(error)
}
