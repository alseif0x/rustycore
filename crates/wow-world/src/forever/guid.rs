//! Target ObjectGuidFactory::CreatePlayer at 02245dcd ObjectGuid.cpp887–912.
use wow_core::ObjectGuid;

#[cfg(test)]
#[path = "../../unit_tests/forever/guid.rs"]
mod tests;

/// Identity admission ensures a nonzero realm; preserve all sixteen realm bits.
/// This raw assembly is distinct from the legacy thirteen-bit GUID factories.
pub(super) fn player_guid(realm_address: u32, counter: u64) -> ObjectGuid {
    ObjectGuid::new(
        ((2_u64 << 58) | (u64::from(realm_address & 0xFFFF) << 42)) as i64,
        counter as i64,
    )
}
