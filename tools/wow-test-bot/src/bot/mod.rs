//! QA bot responsibilities extracted from main.rs under #630.
//!
//! Each submodule owns one operation group; the re-exports below are consumed
//! through the crate root glob, so removing them breaks the callers.

use super::*;

mod character;
mod cli;
mod items_1;
mod items_2;
mod items_3;
mod login;
mod loot;
mod misc_1;
mod misc_2;
mod misc_3;
mod misc_4;
mod misc_5;
mod misc_6;
mod movement;
mod packets;
mod quest;
mod runtime_1;
mod runtime_2;
mod runtime_3;
mod runtime_4;
mod social;
mod spell;
mod sql_fixtures;
mod void_storage_1;
mod void_storage_2;
mod void_storage_3;
mod void_storage_4;

#[allow(unused_imports)]
pub(crate) use character::*;
#[allow(unused_imports)]
pub(crate) use cli::*;
#[allow(unused_imports)]
pub(crate) use items_1::*;
#[allow(unused_imports)]
pub(crate) use items_2::*;
#[allow(unused_imports)]
pub(crate) use items_3::*;
#[allow(unused_imports)]
pub(crate) use login::*;
#[allow(unused_imports)]
pub(crate) use loot::*;
#[allow(unused_imports)]
pub(crate) use misc_1::*;
#[allow(unused_imports)]
pub(crate) use misc_2::*;
#[allow(unused_imports)]
pub(crate) use misc_3::*;
#[allow(unused_imports)]
pub(crate) use misc_4::*;
#[allow(unused_imports)]
pub(crate) use misc_5::*;
#[allow(unused_imports)]
pub(crate) use misc_6::*;
#[allow(unused_imports)]
pub(crate) use movement::*;
#[allow(unused_imports)]
pub(crate) use packets::*;
#[allow(unused_imports)]
pub(crate) use quest::*;
#[allow(unused_imports)]
pub(crate) use runtime_1::*;
#[allow(unused_imports)]
pub(crate) use runtime_2::*;
#[allow(unused_imports)]
pub(crate) use runtime_3::*;
#[allow(unused_imports)]
pub(crate) use runtime_4::*;
#[allow(unused_imports)]
pub(crate) use social::*;
#[allow(unused_imports)]
pub(crate) use spell::*;
#[allow(unused_imports)]
pub(crate) use sql_fixtures::*;
#[allow(unused_imports)]
pub(crate) use void_storage_1::*;
#[allow(unused_imports)]
pub(crate) use void_storage_2::*;
#[allow(unused_imports)]
pub(crate) use void_storage_3::*;
#[allow(unused_imports)]
pub(crate) use void_storage_4::*;
