// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Collection and transmogrification packet definitions.

use wow_constants::{ClientOpcodes, ServerOpcodes};
use wow_core::ObjectGuid;

use crate::world_packet::{PacketError, WorldPacket};
use crate::{ClientPacket, ServerPacket};

pub const COLLECTION_TYPE_TOYBOX_LIKE_CPP: u32 = 1;
pub const COLLECTION_TYPE_APPEARANCE_LIKE_CPP: u32 = 3;
pub const COLLECTION_TYPE_TRANSMOG_SET_LIKE_CPP: u32 = 4;
pub const MAX_TRANSMOGRIFY_ITEMS_LIKE_CPP: usize = 13;

// ── CollectionItemSetFavorite (CMSG 0x3634) ───────────────────────

/// C++ `WorldPackets::Collections::CollectionItemSetFavorite`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollectionItemSetFavorite {
    pub collection_type: u32,
    pub id: u32,
    pub is_favorite: bool,
}

impl ClientPacket for CollectionItemSetFavorite {
    const OPCODE: ClientOpcodes = ClientOpcodes::CollectionItemSetFavorite;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        pkt.skip_opcode();
        let collection_type = pkt.read_uint32()?;
        let id = pkt.read_uint32()?;
        let is_favorite = pkt.read_bit()?;
        Ok(Self {
            collection_type,
            id,
            is_favorite,
        })
    }
}

// ── TransmogrifyItems (CMSG 0xBADD unresolved placeholder) ──────────

/// C++ `WorldPackets::Transmogrification::TransmogrifyItem`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransmogrifyItem {
    pub item_modified_appearance_id: i32,
    pub slot: u32,
    pub spell_item_enchantment_id: i32,
    pub secondary_item_modified_appearance_id: i32,
}

/// C++ `WorldPackets::Transmogrification::TransmogrifyItems`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransmogrifyItems {
    pub npc: ObjectGuid,
    pub items: Vec<TransmogrifyItem>,
    pub current_spec_only: bool,
}

impl TransmogrifyItems {
    /// Reads C++ `TransmogrifyItems::Read`.
    ///
    /// The archived C++ opcode table maps `CMSG_TRANSMOGRIFY_ITEMS` to the
    /// shared `0xBADD` placeholder. Rust cannot register that opcode while it
    /// is also used by other placeholder CMSGs, so production dispatch is
    /// intentionally left unresolved until the real opcode mapping is known.
    pub fn read_like_cpp(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        pkt.skip_opcode();
        let item_count = pkt.read_uint32()? as usize;
        if item_count > MAX_TRANSMOGRIFY_ITEMS_LIKE_CPP {
            return Err(PacketError::TooLarge { size: item_count });
        }

        let npc = pkt.read_packed_guid()?;
        let mut items = Vec::with_capacity(item_count);
        for _ in 0..item_count {
            items.push(TransmogrifyItem {
                item_modified_appearance_id: pkt.read_int32()?,
                slot: pkt.read_uint32()?,
                spell_item_enchantment_id: pkt.read_int32()?,
                secondary_item_modified_appearance_id: pkt.read_int32()?,
            });
        }
        let current_spec_only = pkt.read_bit()?;

        Ok(Self {
            npc,
            items,
            current_spec_only,
        })
    }
}

// ── AccountTransmogUpdate (SMSG 0xBADD placeholder) ───────────────

/// C++ `WorldPackets::Transmogrification::AccountTransmogUpdate`.
///
/// The archived C++ opcode table uses the `0xBADD` placeholder for
/// `SMSG_ACCOUNT_TRANSMOG_UPDATE`. Rust already represents that placeholder as
/// `ServerOpcodes::UpdateCapturePoint`, so this packet keeps a distinct type while
/// sharing the numeric placeholder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountTransmogUpdate {
    pub is_full_update: bool,
    pub is_set_favorite: bool,
    pub favorite_appearances: Vec<u32>,
    pub new_appearances: Vec<u32>,
}

impl AccountTransmogUpdate {
    pub fn favorite_delta(item_modified_appearance_id: u32, is_set_favorite: bool) -> Self {
        Self {
            is_full_update: false,
            is_set_favorite,
            favorite_appearances: vec![item_modified_appearance_id],
            new_appearances: Vec::new(),
        }
    }
}

impl ServerPacket for AccountTransmogUpdate {
    const OPCODE: ServerOpcodes = ServerOpcodes::UpdateCapturePoint;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bit(self.is_full_update);
        pkt.write_bit(self.is_set_favorite);
        pkt.write_uint32(self.favorite_appearances.len() as u32);
        pkt.write_uint32(self.new_appearances.len() as u32);
        for &appearance in &self.favorite_appearances {
            pkt.write_uint32(appearance);
        }
        for &appearance in &self.new_appearances {
            pkt.write_uint32(appearance);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::WorldPacket;

    #[test]
    fn collection_item_set_favorite_reads_cpp_field_order() {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(ClientOpcodes::CollectionItemSetFavorite as u16);
        pkt.write_uint32(COLLECTION_TYPE_APPEARANCE_LIKE_CPP);
        pkt.write_uint32(65);
        pkt.write_bit(true);
        pkt.flush_bits();

        let decoded = CollectionItemSetFavorite::read(&mut pkt).unwrap();
        assert_eq!(
            decoded,
            CollectionItemSetFavorite {
                collection_type: COLLECTION_TYPE_APPEARANCE_LIKE_CPP,
                id: 65,
                is_favorite: true,
            }
        );
    }

    #[test]
    fn transmogrify_items_reads_cpp_field_order() {
        let npc = ObjectGuid::new(0, 0x1234_5678_90ab_cdef);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(0xbadd);
        pkt.write_uint32(2);
        pkt.write_packed_guid(&npc);
        pkt.write_int32(101);
        pkt.write_uint32(4);
        pkt.write_int32(0);
        pkt.write_int32(202);
        pkt.write_int32(-303);
        pkt.write_uint32(8);
        pkt.write_int32(404);
        pkt.write_int32(0);
        pkt.write_bit(true);
        pkt.flush_bits();

        let decoded = TransmogrifyItems::read_like_cpp(&mut pkt).unwrap();
        assert_eq!(decoded.npc, npc);
        assert!(decoded.current_spec_only);
        assert_eq!(
            decoded.items,
            vec![
                TransmogrifyItem {
                    item_modified_appearance_id: 101,
                    slot: 4,
                    spell_item_enchantment_id: 0,
                    secondary_item_modified_appearance_id: 202,
                },
                TransmogrifyItem {
                    item_modified_appearance_id: -303,
                    slot: 8,
                    spell_item_enchantment_id: 404,
                    secondary_item_modified_appearance_id: 0,
                },
            ]
        );
    }

    #[test]
    fn transmogrify_items_rejects_items_over_cpp_max() {
        let npc = ObjectGuid::new(0, 0x1234);
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint16(0xbadd);
        pkt.write_uint32((MAX_TRANSMOGRIFY_ITEMS_LIKE_CPP + 1) as u32);
        pkt.write_packed_guid(&npc);

        assert!(matches!(
            TransmogrifyItems::read_like_cpp(&mut pkt),
            Err(PacketError::TooLarge { size }) if size == MAX_TRANSMOGRIFY_ITEMS_LIKE_CPP + 1
        ));
    }

    #[test]
    fn account_transmog_update_writes_cpp_shape() {
        let bytes = AccountTransmogUpdate {
            is_full_update: true,
            is_set_favorite: false,
            favorite_appearances: vec![65, 96],
            new_appearances: vec![777],
        }
        .to_bytes();

        assert_eq!(
            &bytes[0..2],
            &(ServerOpcodes::UpdateCapturePoint as u16).to_le_bytes()
        );
        assert_eq!(bytes[2], 0b1000_0000);
        assert_eq!(u32::from_le_bytes(bytes[3..7].try_into().unwrap()), 2);
        assert_eq!(u32::from_le_bytes(bytes[7..11].try_into().unwrap()), 1);
        assert_eq!(u32::from_le_bytes(bytes[11..15].try_into().unwrap()), 65);
        assert_eq!(u32::from_le_bytes(bytes[15..19].try_into().unwrap()), 96);
        assert_eq!(u32::from_le_bytes(bytes[19..23].try_into().unwrap()), 777);
    }
}
