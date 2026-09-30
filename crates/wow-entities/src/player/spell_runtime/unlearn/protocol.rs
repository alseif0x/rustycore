//! Typed inputs, owner writes and application effects for one spell removal.
//! No catalog, packet, Session or persistence types belong to this contract.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellUnlearnEdge {
    pub spell_id: u32,
    pub overrides_spell_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellUnlearnOwnerStep {
    Forget { spell_id: i32, preserve_complete: bool },
    DropOverridesAndTrait { spell_id: i32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellUnlearnOwnerOutcome {
    Forgotten { was_dependent: bool },
    TraitDefinition(Option<i32>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellUnlearnStep {
    Known(i32),
    RowsComplete,
    InvalidateRows,
    NextRank(u32),
    Talent(u32),
    Requiring(u32),
    Owner(SpellUnlearnOwnerStep),
    DowngradeSkill(u32),
    Learned(u32),
    RemoveOverride { overridden: i32, replacement: i32 },
    PreviousRank(u32),
    Ranked(u32),
    Reactivate { spell_id: i32, dependent: bool },
    Superceded { spell_id: i32, previous_spell_id: i32 },
    TraitOverride(u32),
    TitanGrip(i32),
    DualWield(i32),
    Offhand,
    Unlearned { spell_id: u32, suppress_messaging: bool },
    Done,
}

pub enum SpellUnlearnInput<I> {
    Known(bool),
    RowsComplete(bool),
    Applied,
    Rank(u32),
    Talent(bool),
    Requiring(Vec<i32>),
    Owner(Option<SpellUnlearnOwnerOutcome>),
    Learned(I),
    Ranked(bool),
    TraitOverride(i32),
}
