//! Actual World/Data adapter integration through public Entity skill operations.
mod activation;
mod admission;
mod catalog_adapter;
mod defaults;
mod fixtures;
mod rewards;
use super::*;
use crate::forever::{
    creation::{SourceError, WorldSources},
    spells::SpellDefinitionSeeds,
};
use fixtures::*;
use wow_data::forever_birth::BirthCatalog;
