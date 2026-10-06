// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::register_instance_handlers_like_cpp;
use crate::instances::{
    InstanceDifficultyHandlerCxLikeCpp, InstanceLockOperationsHandlerCxLikeCpp,
    InstanceRaidInfoHandlerCxLikeCpp, InstancesHandlerHostLikeCpp,
};
use wow_handler::RegistryBuilder;

struct NonCopyHost {
    marker: String,
}

impl InstancesHandlerHostLikeCpp<()> for NonCopyHost {
    fn instance_raid_info_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a (),
    ) -> InstanceRaidInfoHandlerCxLikeCpp<'a> {
        panic!("registration must not construct an instance context")
    }

    fn instance_lock_operations_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a (),
    ) -> InstanceLockOperationsHandlerCxLikeCpp<'a> {
        panic!("registration must not construct an instance context")
    }

    fn instance_difficulty_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a (),
    ) -> InstanceDifficultyHandlerCxLikeCpp<'a> {
        panic!("registration must not construct an instance context")
    }
}

#[test]
fn registrar_adds_entries_without_constructing_host_and_rejects_duplicate() {
    let host = NonCopyHost {
        marker: String::from("still owned by caller"),
    };
    let mut builder = RegistryBuilder::<NonCopyHost, ()>::new();

    assert!(register_instance_handlers_like_cpp(&mut builder).is_ok());
    assert!(register_instance_handlers_like_cpp(&mut builder).is_err());
    assert!(!builder.build().is_empty());
    assert_eq!(host.marker, "still owned by caller");
}
