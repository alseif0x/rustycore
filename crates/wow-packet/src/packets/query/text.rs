//! Text packets.
//!
//! Separated from query.rs under #689.

use super::*;

// ── CMSG_QUERY_PAGE_TEXT (0x3274) ────────────────────────────────────

/// Client request for static page-text data.
pub struct QueryPageText {
    pub page_text_id: u32,
    pub item_guid: ObjectGuid,
}

impl ClientPacket for QueryPageText {
    const OPCODE: ClientOpcodes = ClientOpcodes::QueryPageText;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        let page_text_id = packet.read_uint32()?;
        let guid_bytes = packet.read_bytes(16)?;
        let mut raw = [0_u8; 16];
        raw.copy_from_slice(&guid_bytes);
        Ok(Self {
            page_text_id,
            item_guid: ObjectGuid::from_raw_bytes(&raw),
        })
    }
}

/// One page in `SMSG_QUERY_PAGE_TEXT_RESPONSE`.
pub struct PageTextInfo {
    pub id: u32,
    pub next_page_id: u32,
    pub player_condition_id: i32,
    pub flags: u8,
    pub text: String,
}

/// Static page-text response, including linked pages by `NextPageID`.
pub struct QueryPageTextResponse {
    pub page_text_id: u32,
    pub allow: bool,
    pub pages: Vec<PageTextInfo>,
}

impl ServerPacket for QueryPageTextResponse {
    const OPCODE: ServerOpcodes = ServerOpcodes::QueryPageTextResponse;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.page_text_id);
        pkt.write_bit(self.allow);
        pkt.flush_bits();

        if !self.allow {
            return;
        }

        pkt.write_uint32(self.pages.len() as u32);
        for page in &self.pages {
            pkt.write_uint32(page.id);
            pkt.write_uint32(page.next_page_id);
            pkt.write_int32(page.player_condition_id);
            pkt.write_uint8(page.flags);
            pkt.write_bits(page.text.len() as u32, 12);
            pkt.flush_bits();
            pkt.write_string(&page.text);
        }
    }
}

// ── CMSG_ITEM_TEXT_QUERY (0x32C5) ───────────────────────────────────

/// C++ `WorldPackets::Query::ItemTextQuery`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemTextQuery {
    pub id: ObjectGuid,
}

impl ClientPacket for ItemTextQuery {
    const OPCODE: ClientOpcodes = ClientOpcodes::ItemTextQuery;

    fn read(packet: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            id: packet.read_guid()?,
        })
    }
}

/// C++ `WorldPackets::Query::QueryItemTextResponse`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryItemTextResponse {
    pub id: ObjectGuid,
    pub valid: bool,
    pub text: String,
}

impl QueryItemTextResponse {
    pub fn invalid_like_cpp(id: ObjectGuid) -> Self {
        Self {
            id,
            valid: false,
            text: String::new(),
        }
    }

    pub fn valid_like_cpp(id: ObjectGuid, text: impl Into<String>) -> Self {
        Self {
            id,
            valid: true,
            text: text.into(),
        }
    }
}

impl ServerPacket for QueryItemTextResponse {
    const OPCODE: ServerOpcodes = ServerOpcodes::QueryItemTextResponse;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bit(self.valid);
        pkt.flush_bits();

        pkt.write_bits(self.text.len() as u32, 13);
        pkt.flush_bits();
        pkt.write_string(&self.text);

        pkt.write_guid(&self.id);
    }
}
