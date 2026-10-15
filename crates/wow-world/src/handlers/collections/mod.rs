// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Private collections capability handlers extracted from the legacy misc owner.
//!
//! `#1263 F5 remaining families`: the `CMSG_ADD_TOY` and `CMSG_USE_TOY`
//! registrations moved to the `ApplicationCollections` area registrar
//! (`ToyHandler.cpp:28` and `:54`). The bodies moved to `wow_world_inventory`
//! (`CMSG_ADD_TOY`) and `wow_world_spell` (`CMSG_USE_TOY`) (#1363 slice 4d).

#[cfg(test)]
mod test_shims;

#[cfg(test)]
#[path = "../../../unit_tests/handlers/collections/tests/mod.rs"]
mod tests;
