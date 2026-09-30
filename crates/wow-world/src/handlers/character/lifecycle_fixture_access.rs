//! Narrow test-only Character operation forwards and value projections.

use crate::session::*;
use wow_core::{ObjectGuid, Position};
use wow_packet::packets::{character::*, misc::*, update::*, spell::*};
use wow_data::{PlayerCreateInfoLikeCpp, TaxiPathNodeEntry};
use wow_entities::{CorpseCustomizationChoice, CorpseType};
use std::collections::{BTreeMap, BTreeSet, HashMap};

mod creation;
mod enumeration;
mod location;
mod transport;
mod corpse;
mod operations;
mod constants;

pub use creation::*;
pub use enumeration::*;
pub use location::*;
pub use transport::*;
pub use corpse::*;
pub use constants::*;
