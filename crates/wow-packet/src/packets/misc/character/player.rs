//! Player packets.
//!
//! Separated from character.rs under #689.

use super::*;

/// Opens the barber shop/customization UI for the requested customization scope.
pub struct EnableBarberShop {
    pub customization_scope: u8,
}

impl ServerPacket for EnableBarberShop {
    const OPCODE: ServerOpcodes = ServerOpcodes::EnableBarberShop;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint8(self.customization_scope);
    }
}

// ── UndeleteCooldownStatusResponse (SMSG 0x27ce) ────────────────────

/// Sent after the player's home bind has changed.
///
/// C++ `WorldPackets::Misc::PlayerBound::Write`: ObjectGuid stream + uint32 AreaID.
pub struct PlayerBound {
    pub binder_id: wow_core::ObjectGuid,
    pub area_id: u32,
}

impl ServerPacket for PlayerBound {
    const OPCODE: ServerOpcodes = ServerOpcodes::PlayerBound;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.binder_id);
        pkt.write_uint32(self.area_id);
    }
}

// ── Compact Unit Frame profiles ──────────────────────────────────────

/// Tells the client what weapon/armor types the player can use.
///
/// C++ `WorldPackets::Item::SetProficiency` format:
/// ```text
/// [i32] ProficiencyMask  (bitmask of sub-classes)
/// [u8]  ProficiencyClass (ItemClass enum: 2=Weapon, 4=Armor)
/// ```
pub struct SetProficiency {
    pub proficiency_mask: u32,
    pub proficiency_class: u8,
}

impl ServerPacket for SetProficiency {
    const OPCODE: ServerOpcodes = ServerOpcodes::SetProficiency;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.proficiency_mask);
        pkt.write_uint8(self.proficiency_class);
    }
}

impl SetProficiency {
    /// Default weapon proficiency for a given class.
    ///
    /// Compatibility masks derived from C++ proficiency spell effects.
    /// Class 2 = Weapon (ItemClass.Weapon).
    pub fn default_weapons(class_id: u8) -> Self {
        // Weapon subclass bit positions (1 << subclass):
        //  0=Axe1H     0x00001   7=Sword1H   0x00080   15=Dagger    0x08000
        //  1=Axe2H     0x00002   8=Sword2H   0x00100   16=Thrown    0x10000
        //  2=Bow       0x00004  10=Staff     0x00400   18=Crossbow  0x40000
        //  3=Gun       0x00008  13=Fist      0x02000   19=Wand      0x80000
        //  4=Mace1H    0x00010
        //  5=Mace2H    0x00020
        //  6=Polearm   0x00040
        let mask = match class_id {
            1 => 0x0005_A5FF, // Warrior: Axe12,Bow,Gun,Mace12,Polearm,Sword12,Staff,Fist,Dagger,Thrown,Xbow
            2 => 0x0000_01F3, // Paladin: Axe12,Mace12,Polearm,Sword12
            3 => 0x0005_A5CF, // Hunter: Axe12,Bow,Gun,Polearm,Sword12,Staff,Fist,Dagger,Thrown,Xbow
            4 => 0x0005_A09C, // Rogue: Bow,Gun,Mace1H,Sword1H,Fist,Dagger,Thrown,Xbow
            5 => 0x0008_8410, // Priest: Mace1H,Staff,Dagger,Wand
            6 => 0x0000_01F3, // DK: Axe12,Mace12,Polearm,Sword12
            7 => 0x0000_A433, // Shaman: Axe12,Mace12,Staff,Fist,Dagger
            8 => 0x0008_8480, // Mage: Sword1H,Staff,Dagger,Wand
            9 => 0x0008_8480, // Warlock: Sword1H,Staff,Dagger,Wand
            11 => 0x0000_A470, // Druid: Mace12,Polearm,Staff,Fist,Dagger
            _ => 0x0000_2000, // Fists only
        };
        Self {
            proficiency_mask: mask,
            proficiency_class: 2, // Weapon
        }
    }

    /// Default armor proficiency for a given class.
    ///
    /// Class 4 = Armor (ItemClass.Armor).
    /// Subclass bit positions: Cloth=1(0x02), Leather=2(0x04), Mail=3(0x08),
    /// Plate=4(0x10), Shield=6(0x40).
    pub fn default_armor(class_id: u8) -> Self {
        let mask = match class_id {
            1 => 0x5E,  // Warrior: Cloth+Leather+Mail+Plate+Shield
            2 => 0x5E,  // Paladin: Cloth+Leather+Mail+Plate+Shield
            3 => 0x0E,  // Hunter: Cloth+Leather+Mail
            4 => 0x06,  // Rogue: Cloth+Leather
            5 => 0x02,  // Priest: Cloth
            6 => 0x1E,  // DK: Cloth+Leather+Mail+Plate
            7 => 0x4E,  // Shaman: Cloth+Leather+Mail+Shield
            8 => 0x02,  // Mage: Cloth
            9 => 0x02,  // Warlock: Cloth
            11 => 0x06, // Druid: Cloth+Leather
            _ => 0x02,  // Cloth
        };
        Self {
            proficiency_mask: mask,
            proficiency_class: 4, // Armor
        }
    }
}

/// C++ `WorldPackets::Trade::ClearTradeItem`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ClearTradeItem {
    pub trade_slot: u8,
}

impl ClientPacket for ClearTradeItem {
    const OPCODE: ClientOpcodes = ClientOpcodes::ClearTradeItem;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            trade_slot: pkt.read_uint8()?,
        })
    }
}

/// C++ `WorldPackets::Trade::SetTradeItem`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SetTradeItem {
    pub trade_slot: u8,
    pub pack_slot: u8,
    pub item_slot_in_pack: u8,
}

impl ClientPacket for SetTradeItem {
    const OPCODE: ClientOpcodes = ClientOpcodes::SetTradeItem;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            trade_slot: pkt.read_uint8()?,
            pack_slot: pkt.read_uint8()?,
            item_slot_in_pack: pkt.read_uint8()?,
        })
    }
}
