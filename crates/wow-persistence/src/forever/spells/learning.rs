//! World spell relationships, not hotfix records or a learned Player spellbook.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpellRequiredRow {
    pub spell: u32,
    pub required: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpellLearnRow {
    pub source: u32,
    pub learned: u32,
    pub active: bool,
}
