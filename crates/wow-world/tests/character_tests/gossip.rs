//! Original Character Gossip/Trainer scenarios on the feature fixture rail.

use super::*;
use super::quest_template as character_quest_template;
use std::sync::{Arc, Mutex};
use wow_entities::{GameObject, MapObjectRecord, Player, GAMEOBJECT_TYPE_GOOBER};
use wow_entities::PlayerHomebindLikeCpp as RepresentedHomebindLikeCpp;
use wow_entities::PlayerTaxiFlightNodeLikeCpp as RepresentedTaxiFlightNodeLikeCpp;
use wow_packet::packets::spell::SpellCastVisual;
use wow_world::player_directory::PlayerRegistry;
use wow_world::test_fixtures::{
    make_gossip_bank_session_for_test as make_bank_slot_session,
    insert_gossip_binder_creature_for_test as insert_binder_creature,
};

mod fixtures_world;
mod fixtures_state;
mod gameobjects;
mod binder;
mod admission;
mod binder_fanout;
mod catalogs;
mod services;
mod trainer;
mod quest_text;
mod fixture_modes;
