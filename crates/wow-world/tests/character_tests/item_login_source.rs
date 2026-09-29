#[test]
fn continue_login_inventory_reads_cross_the_typed_lifecycle_port() {
    let handler = // The #1233 decomposition moved the login inventory/repair orchestration into
    // handlers/character/items/login_load.rs; the assertions below are unchanged.
    include_str!("../../src/handlers/character/items/login_load.rs");

    assert!(handler.contains("PlayerLoginAuxiliaryLoadRequestLikeCpp::EquipmentInventory"));
    assert!(handler.contains("PlayerLoginAuxiliaryLoadRequestLikeCpp::BagInventory"));
    assert!(handler.contains("PlayerLoginAuxiliaryLoadRequestLikeCpp::VoidStorage"));
    for statement in [
        "CharStatements::SEL_CHAR_EQUIPMENT",
        "CharStatements::SEL_CHAR_BAG_CONTENTS",
        "CharStatements::SEL_CHAR_VOID_STORAGE",
    ] {
        assert!(
            !handler.contains(statement),
            "handler still names {statement}"
        );
    }
}
#[test]
fn continue_login_item_repairs_cross_the_typed_lifecycle_port() {
    let handler = // The #1233 decomposition moved the login inventory/repair orchestration into
    // handlers/character/items/login_load.rs; the assertions below are unchanged.
    include_str!("../../src/handlers/character/items/login_load.rs");

    assert_eq!(
        handler
            .matches("persist_login_item_repairs_like_cpp")
            .count(),
        2
    );
    assert!(handler.contains("PlayerLoginItemRepairActionLikeCpp::ClearRefundable"));
    assert!(handler.contains("PlayerLoginItemRepairActionLikeCpp::NormalizeOnLoad"));
    assert!(!handler.contains("SqlTransaction::new()"));
    for statement in [
        "CharStatements::DEL_ITEM_REFUND_INSTANCE",
        "CharStatements::UPD_ITEM_INSTANCE_FLAGS",
        "CharStatements::UPD_ITEM_INSTANCE_ON_LOAD",
    ] {
        assert!(
            !handler.contains(statement),
            "handler still names {statement}"
        );
    }
}
