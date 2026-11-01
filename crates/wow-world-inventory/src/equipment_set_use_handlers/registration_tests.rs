// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::{
    EquipmentSetUseContextLikeCpp, EquipmentSetUseHandlerHostLikeCpp,
    register_equipment_set_use_handler_like_cpp,
};
use wow_constants::ClientOpcodes;
use wow_handler::{PacketProcessing, RegistryBuilder, SessionStatus};

struct NonCopyHost {
    marker: String,
}

impl EquipmentSetUseHandlerHostLikeCpp<()> for NonCopyHost {
    fn equipment_set_use_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a (),
    ) -> EquipmentSetUseContextLikeCpp<'a> {
        panic!("registration must not construct an equipment-set context")
    }
}

#[test]
fn registrar_preserves_entry_metadata_without_constructing_host_and_rejects_duplicate() {
    let host = NonCopyHost {
        marker: String::from("still owned by caller"),
    };
    let mut builder = RegistryBuilder::<NonCopyHost, ()>::new();

    assert!(register_equipment_set_use_handler_like_cpp(&mut builder).is_ok());
    assert!(register_equipment_set_use_handler_like_cpp(&mut builder).is_err());
    let registry = builder.build();
    let entry = registry.get(ClientOpcodes::UseEquipmentSet).unwrap();
    assert_eq!(entry.status, SessionStatus::LoggedIn);
    assert_eq!(entry.processing, PacketProcessing::Inplace);
    assert_eq!(entry.handler_name, "handle_use_equipment_set");
    assert_eq!(registry.len(), 1);
    assert_eq!(host.marker, "still owned by caller");
}
