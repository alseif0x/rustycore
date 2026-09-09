//! Movement info and packets regression scenarios.
//!
//! Separated from the movement.rs root under #650.

use super::*;
use crate::world_packet::WorldPacket;
use wow_core::guid::HighGuid;
use wow_movement::{AnimTierTransition, FacingInfo, MoveSplineInitArgs, SpellEffectExtraData};

mod scenarios_1;
