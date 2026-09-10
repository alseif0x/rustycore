//! Battle pet ops packets.
//!
//! Separated from world_state.rs under #689.

use super::*;

/// C++ `WorldPackets::BattlePet::BattlePetSetBattleSlot`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattlePetSetBattleSlot {
    pub pet_guid: ObjectGuid,
    pub slot: u8,
}

impl ClientPacket for BattlePetSetBattleSlot {
    const OPCODE: ClientOpcodes = ClientOpcodes::BattlePetSetBattleSlot;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        pkt.skip_opcode();
        Ok(Self {
            pet_guid: pkt.read_packed_guid()?,
            slot: pkt.read_uint8()?,
        })
    }
}

/// C++ `WorldPackets::BattlePet::BattlePetSummon`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattlePetSummon {
    pub pet_guid: ObjectGuid,
}

impl ClientPacket for BattlePetSummon {
    const OPCODE: ClientOpcodes = ClientOpcodes::BattlePetSummon;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        pkt.skip_opcode();
        Ok(Self {
            pet_guid: pkt.read_packed_guid()?,
        })
    }
}

/// C++ `WorldPackets::BattlePet::BattlePetUpdateNotify`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattlePetUpdateNotify {
    pub pet_guid: ObjectGuid,
}

impl ClientPacket for BattlePetUpdateNotify {
    const OPCODE: ClientOpcodes = ClientOpcodes::BattlePetUpdateNotify;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        pkt.skip_opcode();
        Ok(Self {
            pet_guid: pkt.read_packed_guid()?,
        })
    }
}

/// C++ `WorldPackets::BattlePet::BattlePetDeletePet`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattlePetDeletePet {
    pub pet_guid: ObjectGuid,
}

impl BattlePetDeletePet {
    /// Reads C++ `BattlePetDeletePet::Read`.
    ///
    /// The archived C++ opcode table maps `CMSG_BATTLE_PET_DELETE_PET` to the
    /// shared `0xBADD` placeholder. Rust must not register production dispatch
    /// until the real opcode mapping is known.
    pub fn read_like_cpp(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        pkt.skip_opcode();
        Ok(Self {
            pet_guid: pkt.read_packed_guid()?,
        })
    }
}

/// C++ `WorldPackets::BattlePet::CageBattlePet`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CageBattlePet {
    pub pet_guid: ObjectGuid,
}

impl CageBattlePet {
    /// Reads C++ `CageBattlePet::Read`.
    ///
    /// The archived C++ opcode table maps `CMSG_CAGE_BATTLE_PET` to the shared
    /// `0xBADD` placeholder. Rust must not register production dispatch until
    /// the real opcode mapping is known.
    pub fn read_like_cpp(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        pkt.skip_opcode();
        Ok(Self {
            pet_guid: pkt.read_packed_guid()?,
        })
    }
}

/// C++ `WorldPackets::BattlePet::BattlePetModifyName`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattlePetModifyName {
    pub pet_guid: ObjectGuid,
    pub name: String,
    pub declined_names: Option<DeclinedNamesLikeCpp>,
}

impl BattlePetModifyName {
    /// Reads C++ `BattlePetModifyName::Read`.
    ///
    /// The archived C++ opcode table maps `CMSG_BATTLE_PET_MODIFY_NAME` to the
    /// shared `0xBADD` placeholder. Rust must not register production dispatch
    /// until the real opcode mapping is known.
    pub fn read_like_cpp(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        pkt.skip_opcode();
        let pet_guid = pkt.read_packed_guid()?;
        let name_length = pkt.read_bits(7)? as usize;
        let has_declined_names = pkt.read_bit()?;

        let declined_names = if has_declined_names {
            let mut lengths = [0usize; MAX_DECLINED_NAME_CASES_LIKE_CPP];
            for length in &mut lengths {
                *length = pkt.read_bits(7)? as usize;
            }

            let names_vec: Vec<String> = lengths
                .iter()
                .map(|length| pkt.read_string(*length))
                .collect::<Result<_, _>>()?;
            let names: [String; MAX_DECLINED_NAME_CASES_LIKE_CPP] =
                names_vec.try_into().map_err(|_| PacketError::TooLarge {
                    size: MAX_DECLINED_NAME_CASES_LIKE_CPP + 1,
                })?;
            Some(DeclinedNamesLikeCpp { names })
        } else {
            None
        };

        let name = pkt.read_string(name_length)?;

        Ok(Self {
            pet_guid,
            name,
            declined_names,
        })
    }
}

/// C++ `WorldPackets::BattlePet::QueryBattlePetName`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueryBattlePetName {
    pub battle_pet_id: ObjectGuid,
    pub unit_guid: ObjectGuid,
}

impl ClientPacket for QueryBattlePetName {
    const OPCODE: ClientOpcodes = ClientOpcodes::QueryBattlePetName;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        pkt.skip_opcode();
        Ok(Self {
            battle_pet_id: pkt.read_packed_guid()?,
            unit_guid: pkt.read_packed_guid()?,
        })
    }
}

/// C++ `WorldPackets::BattlePet::QueryBattlePetNameResponse`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryBattlePetNameResponse {
    pub battle_pet_id: ObjectGuid,
    pub creature_id: i32,
    pub timestamp: i64,
    pub allow: bool,
    pub name: String,
    pub declined_names: Option<DeclinedNamesLikeCpp>,
}

impl QueryBattlePetNameResponse {
    pub fn not_allowed(battle_pet_id: ObjectGuid) -> Self {
        Self {
            battle_pet_id,
            creature_id: 0,
            timestamp: 0,
            allow: false,
            name: String::new(),
            declined_names: None,
        }
    }

    pub fn allowed(
        battle_pet_id: ObjectGuid,
        creature_id: i32,
        timestamp: i64,
        name: String,
        declined_names: Option<DeclinedNamesLikeCpp>,
    ) -> Self {
        Self {
            battle_pet_id,
            creature_id,
            timestamp,
            allow: true,
            name,
            declined_names,
        }
    }
}

impl ServerPacket for QueryBattlePetNameResponse {
    const OPCODE: ServerOpcodes = ServerOpcodes::QueryBattlePetNameResponse;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.battle_pet_id);
        pkt.write_int32(self.creature_id);
        pkt.write_int64(self.timestamp);
        pkt.write_bit(self.allow);
        if self.allow {
            pkt.write_bits(self.name.len() as u32, 8);
            pkt.write_bit(self.declined_names.is_some());

            let declined_names = self.declined_names.as_ref().map(|declined| &declined.names);
            for index in 0..MAX_DECLINED_NAME_CASES_LIKE_CPP {
                let length = declined_names
                    .map(|names| names[index].len())
                    .unwrap_or_default();
                pkt.write_bits(length as u32, 7);
            }

            if let Some(names) = declined_names {
                for name in names {
                    pkt.write_string(name);
                }
            }
            pkt.write_string(&self.name);
        }
        pkt.flush_bits();
    }
}

/// C++ `WorldPackets::BattlePet::BattlePetSetFlags`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattlePetSetFlags {
    pub pet_guid: ObjectGuid,
    pub flags: u16,
    pub control_type: u8,
}

impl ClientPacket for BattlePetSetFlags {
    const OPCODE: ClientOpcodes = ClientOpcodes::BattlePetSetFlags;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        pkt.skip_opcode();
        let pet_guid = pkt.read_packed_guid()?;
        let flags = pkt.read_uint16()?;
        let control_type = pkt.read_bits(2)? as u8;
        Ok(Self {
            pet_guid,
            flags,
            control_type,
        })
    }
}

/// C++ `WorldPackets::BattlePet::BattlePetClearFanfare`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattlePetClearFanfare {
    pub pet_guid: ObjectGuid,
}

impl ClientPacket for BattlePetClearFanfare {
    const OPCODE: ClientOpcodes = ClientOpcodes::BattlePetClearFanfare;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        pkt.skip_opcode();
        Ok(Self {
            pet_guid: pkt.read_packed_guid()?,
        })
    }
}

/// C++ `WorldPackets::BattlePet::BattlePetDeleted`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattlePetDeleted {
    pub pet_guid: ObjectGuid,
}

impl ServerPacket for BattlePetDeleted {
    const OPCODE: ServerOpcodes = ServerOpcodes::BattlePetDeleted;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.pet_guid);
    }
}

/// C++ `BattlePets::BattlePetError` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattlePetErrorCodeLikeCpp {
    CantHaveMorePetsOfType = 3,
    CantHaveMorePets = 4,
    TooHighLevelToUncage = 7,
}

/// C++ `WorldPackets::BattlePet::BattlePetError`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattlePetError {
    pub result: u8,
    pub creature_id: i32,
}

impl BattlePetError {
    pub fn new(result: BattlePetErrorCodeLikeCpp, creature_id: i32) -> Self {
        Self {
            result: result as u8,
            creature_id,
        }
    }
}

impl ServerPacket for BattlePetError {
    const OPCODE: ServerOpcodes = ServerOpcodes::BattlePetError;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_bits(self.result as u32, 4);
        pkt.write_int32(self.creature_id);
    }
}
