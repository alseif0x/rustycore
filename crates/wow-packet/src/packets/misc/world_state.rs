// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World state, instance, quest and world-object packets.

use super::*;

/// C++ `WorldPackets::Misc::SetDungeonDifficulty`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetDungeonDifficulty {
    pub difficulty_id: u32,
}

impl ClientPacket for SetDungeonDifficulty {
    const OPCODE: ClientOpcodes = ClientOpcodes::SetDungeonDifficulty;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            difficulty_id: pkt.read_uint32()?,
        })
    }
}

/// C++ `WorldPackets::Misc::SetRaidDifficulty`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetRaidDifficulty {
    pub difficulty_id: i32,
    pub legacy: u8,
}

impl ClientPacket for SetRaidDifficulty {
    const OPCODE: ClientOpcodes = ClientOpcodes::SetRaidDifficulty;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            difficulty_id: pkt.read_int32()?,
            legacy: pkt.read_uint8()?,
        })
    }
}

/// C++ `WorldPackets::Misc::SetDifficultyId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetDifficultyId {
    pub difficulty_id: u32,
}

impl ClientPacket for SetDifficultyId {
    const OPCODE: ClientOpcodes = ClientOpcodes::SetDifficultyId;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            difficulty_id: pkt.read_uint32()?,
        })
    }
}

/// C++ `WorldPackets::Null` for `CMSG_TOGGLE_DIFFICULTY`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToggleDifficulty;

impl ClientPacket for ToggleDifficulty {
    const OPCODE: ClientOpcodes = ClientOpcodes::ToggleDifficulty;

    fn read(_pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self)
    }
}

/// C++ `WorldPackets::WorldState::UpdateWorldState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpdateWorldState {
    pub variable_id: u32,
    pub value: i32,
    pub hidden: bool,
}

impl UpdateWorldState {
    pub fn new(variable_id: u32, value: i32) -> Self {
        Self {
            variable_id,
            value,
            hidden: false,
        }
    }
}

impl ServerPacket for UpdateWorldState {
    const OPCODE: ServerOpcodes = ServerOpcodes::UpdateWorldState;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.variable_id);
        pkt.write_int32(self.value);
        pkt.write_bit(self.hidden);
        pkt.flush_bits();
    }
}

// ── FishNotHooked (SMSG 0x26cf) ─────────────────────────────────────

/// Opens a gameobject-backed interaction UI.
pub struct GameObjectInteraction {
    pub object_guid: ObjectGuid,
    pub interaction_type: i32,
}

impl ServerPacket for GameObjectInteraction {
    const OPCODE: ServerOpcodes = ServerOpcodes::GameObjectInteraction;

    fn write(&self, pkt: &mut WorldPacket) {
        for byte in self.object_guid.to_raw_bytes() {
            pkt.write_uint8(byte);
        }
        pkt.write_int32(self.interaction_type);
    }
}

// ── GameObjectCustomAnim (SMSG 0x25c4) ───────────────────────────────

/// Broadcasts a custom animation for a gameobject.
pub struct GameObjectCustomAnim {
    pub object_guid: ObjectGuid,
    pub custom_anim: u32,
    pub play_as_despawn: bool,
}

impl ServerPacket for GameObjectCustomAnim {
    const OPCODE: ServerOpcodes = ServerOpcodes::GameObjectCustomAnim;

    fn write(&self, pkt: &mut WorldPacket) {
        for byte in self.object_guid.to_raw_bytes() {
            pkt.write_uint8(byte);
        }
        pkt.write_uint32(self.custom_anim);
        pkt.write_bit(self.play_as_despawn);
        pkt.flush_bits();
    }
}

// ── GameObjectDespawn (SMSG 0x25c5) ─────────────────────────────────

/// Notifies the client that a gameobject despawned.
pub struct GameObjectDespawn {
    pub object_guid: ObjectGuid,
}

impl ServerPacket for GameObjectDespawn {
    const OPCODE: ServerOpcodes = ServerOpcodes::GameObjectDespawn;

    fn write(&self, pkt: &mut WorldPacket) {
        for byte in self.object_guid.to_raw_bytes() {
            pkt.write_uint8(byte);
        }
    }
}

// ── CapturePointRemoved (SMSG 0xbadd/UNKNOWN placeholder) ────────────

/// C++ `WorldPackets::Battleground::CapturePointRemoved`.
///
/// The legacy C++ opcode table still marks this battleground packet as
/// `0xBADD`; the archived TrinityCore source marks it as `UNKNOWN_OPCODE` too.
/// Rust cannot model two `ServerOpcodes` enum variants with the same numeric
/// placeholder, so this serializer intentionally shares the current
/// `UpdateCapturePoint` placeholder while preserving the distinct packet type
/// and payload shape.
pub struct CapturePointRemoved {
    pub capture_point_guid: ObjectGuid,
}

impl ServerPacket for CapturePointRemoved {
    const OPCODE: ServerOpcodes = ServerOpcodes::UpdateCapturePoint;

    fn write(&self, pkt: &mut WorldPacket) {
        for byte in self.capture_point_guid.to_raw_bytes() {
            pkt.write_uint8(byte);
        }
    }
}

// ── GameObjectSetStateLocal (SMSG 0x2806) ───────────────────────────

/// Sets a gameobject state only for the receiving client.
pub struct GameObjectSetStateLocal {
    pub object_guid: ObjectGuid,
    pub state: u8,
}

impl ServerPacket for GameObjectSetStateLocal {
    const OPCODE: ServerOpcodes = ServerOpcodes::GameObjectSetStateLocal;

    fn write(&self, pkt: &mut WorldPacket) {
        for byte in self.object_guid.to_raw_bytes() {
            pkt.write_uint8(byte);
        }
        pkt.write_uint8(self.state);
    }
}

// ── AnimKit control packets ────────────────────────────────────────

/// C++ `WorldPackets::Battleground::UpdateCapturePoint`.
pub struct UpdateCapturePoint {
    pub guid: ObjectGuid,
    pub position: Position,
    pub state: u8,
    pub capture_time_ms: u32,
    pub capture_total_duration_ms: u32,
}

impl ServerPacket for UpdateCapturePoint {
    const OPCODE: ServerOpcodes = ServerOpcodes::UpdateCapturePoint;

    fn write(&self, pkt: &mut WorldPacket) {
        for byte in self.guid.to_raw_bytes() {
            pkt.write_uint8(byte);
        }
        pkt.write_float(self.position.x);
        pkt.write_float(self.position.y);
        pkt.write_uint8(self.state);

        if matches!(self.state, 2 | 3) {
            pkt.write_uint32(self.capture_time_ms);
            pkt.write_uint32(self.capture_total_duration_ms);
        }
    }
}

// ── PageText (SMSG 0x2719) ───────────────────────────────────────────

/// Time zone info sent to the client.
pub struct SetTimeZoneInformation {
    pub server_timezone: String,
    pub game_timezone: String,
    pub server_regional_timezone: String,
}

impl SetTimeZoneInformation {
    pub fn utc() -> Self {
        Self {
            server_timezone: "Etc/UTC".into(),
            game_timezone: "Etc/UTC".into(),
            server_regional_timezone: "Etc/UTC".into(),
        }
    }
}

impl ServerPacket for SetTimeZoneInformation {
    const OPCODE: ServerOpcodes = ServerOpcodes::SetTimeZoneInformation;

    fn write(&self, pkt: &mut WorldPacket) {
        // 7-bit length-prefixed strings
        pkt.write_bits(self.server_timezone.len() as u32, 7);
        pkt.write_bits(self.game_timezone.len() as u32, 7);
        pkt.write_bits(self.server_regional_timezone.len() as u32, 7);
        pkt.flush_bits();

        pkt.write_string(&self.server_timezone);
        pkt.write_string(&self.game_timezone);
        pkt.write_string(&self.server_regional_timezone);
    }
}

// ── LoginSetTimeSpeed (SMSG 0x270d) ─────────────────────────────────

/// World state variables for the current zone. Sent after UpdateObject.
/// C++ `WorldPackets::WorldState::InitWorldStates`.
pub struct InitWorldStates {
    pub map_id: i32,
    pub area_id: i32,
    pub subarea_id: i32,
    pub world_states: Vec<(i32, i32)>,
}

impl InitWorldStates {
    pub fn new(map_id: i32, zone_id: i32) -> Self {
        Self {
            map_id,
            area_id: zone_id,
            subarea_id: 0,
            world_states: Vec::new(),
        }
    }

    pub fn with_world_states(
        map_id: i32,
        zone_id: i32,
        area_id: i32,
        world_states: Vec<(i32, i32)>,
    ) -> Self {
        Self {
            map_id,
            area_id: zone_id,
            subarea_id: area_id,
            world_states,
        }
    }
}

impl ServerPacket for InitWorldStates {
    const OPCODE: ServerOpcodes = ServerOpcodes::InitWorldStates;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.map_id);
        pkt.write_int32(self.area_id);
        pkt.write_int32(self.subarea_id);
        pkt.write_uint32(self.world_states.len() as u32);
        for (variable_id, value) in &self.world_states {
            pkt.write_int32(*variable_id);
            pkt.write_int32(*value);
        }
    }
}

// ── UpdateTalentData (SMSG 0x25d7) ──────────────────────────────────

/// Hearthstone bind point. Sent during login.
pub struct BindPointUpdate {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub map_id: u32,
    pub area_id: u32,
}

impl ServerPacket for BindPointUpdate {
    const OPCODE: ServerOpcodes = ServerOpcodes::BindPointUpdate;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_float(self.x);
        pkt.write_float(self.y);
        pkt.write_float(self.z);
        pkt.write_uint32(self.map_id);
        pkt.write_uint32(self.area_id);
    }
}

// ── PlayerBound (SMSG 0x2ff8) ───────────────────────────────────────

/// World server info sent during login.
pub struct WorldServerInfo {
    pub difficulty_id: i32,
}

impl WorldServerInfo {
    pub fn default_open_world() -> Self {
        Self { difficulty_id: 0 }
    }
}

impl ServerPacket for WorldServerInfo {
    const OPCODE: ServerOpcodes = ServerOpcodes::WorldServerInfo;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.difficulty_id);
        pkt.write_bit(false); // IsTournamentRealm
        pkt.write_bit(false); // XRealmPvpAlert
        pkt.write_bit(false); // RestrictedAccountMaxLevel.HasValue
        pkt.write_bit(false); // RestrictedAccountMaxMoney.HasValue
        pkt.write_bit(false); // InstanceGroupSize.HasValue
        pkt.flush_bits();
        // No optional fields written (all HasValue=false)
    }
}

// ── InitialSetup (SMSG 0x2580) ─────────────────────────────────────

/// Account-wide achievement criteria. Empty for fresh accounts.
pub struct AllAccountCriteria;

impl ServerPacket for AllAccountCriteria {
    const OPCODE: ServerOpcodes = ServerOpcodes::AllAccountCriteria;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(0); // Progress.Count
    }
}

// ── AllAchievementData (SMSG 0x2570) ─────────────────────────────────

/// Account-wide achievements. Empty for fresh accounts.
pub struct AllAchievementData;

impl ServerPacket for AllAchievementData {
    const OPCODE: ServerOpcodes = ServerOpcodes::AllAchievementData;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(0); // Earned.Count
        pkt.write_int32(0); // Progress.Count
    }
}

// ── AccountMountUpdate (SMSG 0x25ae) ─────────────────────────────────

/// C++ `WorldPackets::BattlePet::BattlePetRequestJournal`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattlePetRequestJournal;

impl ClientPacket for BattlePetRequestJournal {
    const OPCODE: ClientOpcodes = ClientOpcodes::BattlePetRequestJournal;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        pkt.skip_opcode();
        Ok(Self)
    }
}

/// C++ `WorldPackets::BattlePet::BattlePetRequestJournalLock`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattlePetRequestJournalLock;

impl ClientPacket for BattlePetRequestJournalLock {
    const OPCODE: ClientOpcodes = ClientOpcodes::BattlePetRequestJournalLock;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        pkt.skip_opcode();
        Ok(Self)
    }
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattlePetJournalSlot {
    pub pet_guid: ObjectGuid,
    pub collar_id: u32,
    pub index: u8,
    pub locked: bool,
}

impl BattlePetJournalSlot {
    pub fn locked_empty(index: u8) -> Self {
        Self {
            pet_guid: empty_battle_pet_guid_like_cpp(),
            collar_id: 0,
            index,
            locked: true,
        }
    }
}

/// C++ `WorldPackets::BattlePet::BattlePetOwnerInfo`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattlePetJournalPetOwnerInfo {
    pub guid: ObjectGuid,
    pub player_virtual_realm: u32,
    pub player_native_realm: u32,
}

/// C++ `WorldPackets::BattlePet::BattlePet`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattlePetJournalPet {
    pub guid: ObjectGuid,
    pub species: u32,
    pub creature_id: u32,
    pub display_id: u32,
    pub breed: u16,
    pub level: u16,
    pub exp: u16,
    pub flags: u16,
    pub power: u32,
    pub health: u32,
    pub max_health: u32,
    pub speed: u32,
    pub quality: u8,
    pub owner_info: Option<BattlePetJournalPetOwnerInfo>,
    pub name: String,
}

impl BattlePetJournalPet {
    fn write_like_cpp(&self, pkt: &mut WorldPacket) {
        pkt.write_packed_guid(&self.guid);
        pkt.write_uint32(self.species);
        pkt.write_uint32(self.creature_id);
        pkt.write_uint32(self.display_id);
        pkt.write_uint16(self.breed);
        pkt.write_uint16(self.level);
        pkt.write_uint16(self.exp);
        pkt.write_uint16(self.flags);
        pkt.write_uint32(self.power);
        pkt.write_uint32(self.health);
        pkt.write_uint32(self.max_health);
        pkt.write_uint32(self.speed);
        pkt.write_uint8(self.quality);
        pkt.write_bits(self.name.len() as u32, 7);
        pkt.write_bit(self.owner_info.is_some());
        pkt.write_bit(false); // NoRename
        pkt.flush_bits();
        pkt.write_string(&self.name);

        if let Some(owner_info) = self.owner_info {
            pkt.write_packed_guid(&owner_info.guid);
            pkt.write_uint32(owner_info.player_virtual_realm);
            pkt.write_uint32(owner_info.player_native_realm);
        }
    }
}

/// C++ `WorldPackets::BattlePet::BattlePetJournal`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattlePetJournal {
    pub trap: u16,
    pub has_journal_lock: bool,
    pub slots: Vec<BattlePetJournalSlot>,
    pub pets: Vec<BattlePetJournalPet>,
}

impl BattlePetJournal {
    pub fn empty_with_default_slots(has_journal_lock: bool) -> Self {
        Self {
            trap: 0,
            has_journal_lock,
            slots: (0..3).map(BattlePetJournalSlot::locked_empty).collect(),
            pets: Vec::new(),
        }
    }
}

impl ServerPacket for BattlePetJournal {
    const OPCODE: ServerOpcodes = ServerOpcodes::BattlePetJournal;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint16(self.trap);
        pkt.write_uint32(self.slots.len() as u32);
        pkt.write_uint32(self.pets.len() as u32);
        pkt.write_bit(self.has_journal_lock);
        pkt.flush_bits();

        for slot in &self.slots {
            pkt.write_packed_guid(&slot.pet_guid);
            pkt.write_uint32(slot.collar_id);
            pkt.write_uint8(slot.index);
            pkt.write_bit(slot.locked);
            pkt.flush_bits();
        }

        for pet in &self.pets {
            pet.write_like_cpp(pkt);
        }
    }
}

/// C++ `WorldPackets::BattlePet::BattlePetUpdates`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattlePetUpdates {
    pub pets: Vec<BattlePetJournalPet>,
    pub pet_added: bool,
}

impl ServerPacket for BattlePetUpdates {
    const OPCODE: ServerOpcodes = ServerOpcodes::BattlePetUpdates;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.pets.len() as u32);
        pkt.write_bit(self.pet_added);
        pkt.flush_bits();

        for pet in &self.pets {
            pet.write_like_cpp(pkt);
        }
    }
}

/// C++ `WorldPackets::BattlePet::PetBattleSlotUpdates`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PetBattleSlotUpdates {
    pub slots: Vec<BattlePetJournalSlot>,
    pub auto_slotted: bool,
    pub new_slot: bool,
}

impl ServerPacket for PetBattleSlotUpdates {
    const OPCODE: ServerOpcodes = ServerOpcodes::PetBattleSlotUpdates;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.slots.len() as u32);
        pkt.write_bit(self.new_slot);
        pkt.write_bit(self.auto_slotted);
        pkt.flush_bits();

        for slot in &self.slots {
            pkt.write_packed_guid(&slot.pet_guid);
            pkt.write_uint32(slot.collar_id);
            pkt.write_uint8(slot.index);
            pkt.write_bit(slot.locked);
            pkt.flush_bits();
        }
    }
}

/// Tells the client that the battle pet journal lock has been acquired.
/// Empty packet (opcode only, no payload).
pub struct BattlePetJournalLockAcquired;

impl ServerPacket for BattlePetJournalLockAcquired {
    const OPCODE: ServerOpcodes = ServerOpcodes::BattlePetJournalLockAcquired;

    fn write(&self, _pkt: &mut WorldPacket) {
        // Empty packet — no payload
    }
}

/// Tells the client that the battle pet journal lock was denied.
/// Empty packet (opcode only, no payload).
pub struct BattlePetJournalLockDenied;

impl ServerPacket for BattlePetJournalLockDenied {
    const OPCODE: ServerOpcodes = ServerOpcodes::BattlePetJournalLockDenied;

    fn write(&self, _pkt: &mut WorldPacket) {
        // Empty packet — no payload
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

// ── DungeonDifficultySet (SMSG 0x26a4) ───────────────────────────────

/// Sets the current dungeon difficulty. Sent BEFORE LoginVerifyWorld.
/// C# sends this via `Player.SendDungeonDifficulty()` during HandlePlayerLogin.
pub struct DungeonDifficultySet {
    pub difficulty_id: i32,
}

impl DungeonDifficultySet {
    /// Normal dungeon difficulty (default for fresh characters).
    pub fn normal() -> Self {
        Self { difficulty_id: 0 }
    }
}

impl ServerPacket for DungeonDifficultySet {
    const OPCODE: ServerOpcodes = ServerOpcodes::SetDungeonDifficulty;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.difficulty_id);
    }
}

// ── RaidDifficultySet (SMSG 0x27ad) ──────────────────────────────────

/// Sets the current raid difficulty.
///
/// C++ `WorldPackets::Misc::RaidDifficultySet::Write`:
/// `int32 DifficultyID` followed by `uint8 Legacy`.
pub struct RaidDifficultySet {
    pub difficulty_id: i32,
    pub legacy: bool,
}

impl ServerPacket for RaidDifficultySet {
    const OPCODE: ServerOpcodes = ServerOpcodes::RaidDifficultySet;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_int32(self.difficulty_id);
        pkt.write_uint8(u8::from(self.legacy));
    }
}

// ── DbQueryBulk (CMSG 0x35e5) ─────────────────────────────────────

/// Sent after WorldPortResponse to place the player in the new world.
/// C# ref: MovementPackets.NewWorld
pub struct NewWorld {
    pub map_id: u32,
    pub pos: wow_core::Position,
    /// 0 = Normal teleport, 1 = Seamless.
    pub reason: u32,
}

impl ServerPacket for NewWorld {
    const OPCODE: ServerOpcodes = ServerOpcodes::NewWorld;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.map_id);
        // TeleportLocation: Pos (XYZO) + two unused int32 fields (-1, -1)
        pkt.write_float(self.pos.x);
        pkt.write_float(self.pos.y);
        pkt.write_float(self.pos.z);
        pkt.write_float(self.pos.orientation);
        pkt.write_int32(-1); // Unused901_1
        pkt.write_int32(-1); // Unused901_2
        pkt.write_uint32(self.reason);
        // MovementOffset (all zeros)
        pkt.write_float(0.0);
        pkt.write_float(0.0);
        pkt.write_float(0.0);
    }
}

// ── TransferAborted (SMSG 0x2703) ───────────────────────────────────

/// C++ `WorldPackets::NPC::RequestStabledPets`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestStabledPets {
    pub stable_master: ObjectGuid,
}

impl ClientPacket for RequestStabledPets {
    const OPCODE: wow_constants::ClientOpcodes = wow_constants::ClientOpcodes::RequestStabledPets;

    fn read(pkt: &mut crate::WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            stable_master: pkt.read_packed_guid()?,
        })
    }
}

/// C++ `WorldPackets::LFG::LFGUpdateStatus`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LfgUpdateStatus {
    pub ticket: LfgRideTicket,
    pub sub_type: u8,
    pub reason: u8,
    pub slots: Vec<u32>,
    pub requested_roles: u8,
    pub suspended_players: Vec<ObjectGuid>,
    pub queue_map_id: u32,
    pub notify_ui: bool,
    pub is_party: bool,
    pub joined: bool,
    pub lfg_joined: bool,
    pub queued: bool,
    pub unused: bool,
}

impl LfgUpdateStatus {
    /// C++ `HandleLfgListGetStatus` branch when `sLFGMgr` has no state/ticket.
    pub fn removed_from_queue() -> Self {
        Self {
            ticket: LfgRideTicket::default(),
            sub_type: LFG_QUEUE_DUNGEON_LIKE_CPP,
            reason: LFG_UPDATE_TYPE_REMOVED_FROM_QUEUE_LIKE_CPP,
            slots: Vec::new(),
            requested_roles: 0,
            suspended_players: Vec::new(),
            queue_map_id: 0,
            notify_ui: true,
            is_party: false,
            joined: false,
            lfg_joined: false,
            queued: false,
            unused: false,
        }
    }
}

impl ServerPacket for LfgUpdateStatus {
    const OPCODE: ServerOpcodes = ServerOpcodes::LfgUpdateStatus;

    fn write(&self, pkt: &mut WorldPacket) {
        self.ticket.write_like_cpp(pkt);
        pkt.write_uint8(self.sub_type);
        pkt.write_uint8(self.reason);
        pkt.write_uint32(self.slots.len() as u32);
        pkt.write_uint8(self.requested_roles);
        pkt.write_uint32(self.suspended_players.len() as u32);
        pkt.write_uint32(self.queue_map_id);

        for slot in &self.slots {
            pkt.write_uint32(*slot);
        }

        for suspended_player in &self.suspended_players {
            pkt.write_packed_guid(suspended_player);
        }

        pkt.write_bit(self.is_party);
        pkt.write_bit(self.notify_ui);
        pkt.write_bit(self.joined);
        pkt.write_bit(self.lfg_joined);
        pkt.write_bit(self.queued);
        pkt.write_bit(self.unused);
        pkt.flush_bits();
    }
}

/// C++ `WorldPackets::LFG::LFGListBlacklist::BlacklistEntry`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LfgListBlacklistEntry {
    pub slot: u32,
    pub reason: u32,
    pub sub_reason1: i32,
    pub sub_reason2: i32,
    pub soft_lock: u32,
}

/// C++ `WorldPackets::LFG::LFGListBlacklist`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LfgListBlacklist {
    pub entries: Vec<LfgListBlacklistEntry>,
}

impl LfgListBlacklist {
    pub fn empty() -> Self {
        Self::default()
    }
}

impl ServerPacket for LfgListBlacklist {
    const OPCODE: ServerOpcodes = ServerOpcodes::LfgListUpdateBlacklist;

    fn write(&self, pkt: &mut WorldPacket) {
        pkt.write_uint32(self.entries.len() as u32);
        for entry in &self.entries {
            pkt.write_uint32(entry.slot);
            pkt.write_uint32(entry.reason);
            pkt.write_int32(entry.sub_reason1);
            pkt.write_int32(entry.sub_reason2);
            pkt.write_uint32(entry.soft_lock);
        }
    }
}

/// C++ `WorldPackets::LFG::LFGBlackList`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LfgBlackList {
    pub player_guid: Option<ObjectGuid>,
    pub slots: Vec<LfgListBlacklistEntry>,
}

impl LfgBlackList {
    pub(super) fn write_like_cpp(&self, pkt: &mut WorldPacket) {
        pkt.write_bit(self.player_guid.is_some());
        pkt.write_uint32(self.slots.len() as u32);
        if let Some(player_guid) = self.player_guid {
            pkt.write_packed_guid(&player_guid);
        }
        for slot in &self.slots {
            pkt.write_uint32(slot.slot);
            pkt.write_uint32(slot.reason);
            pkt.write_int32(slot.sub_reason1);
            pkt.write_int32(slot.sub_reason2);
            pkt.write_uint32(slot.soft_lock);
        }
    }
}

/// C++ `WorldPackets::Calendar::SetSavedInstanceExtend`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetSavedInstanceExtend {
    pub map_id: i32,
    pub difficulty_id: u32,
    pub extend: bool,
}

impl ClientPacket for SetSavedInstanceExtend {
    // The inspected TrinityCore 3.4.3 opcode table uses the shared unresolved
    // `0xBADD` placeholder for `CMSG_SET_SAVED_INSTANCE_EXTEND`,
    // `CMSG_SET_LOOT_SPECIALIZATION`, and `CMSG_CLEAR_RAID_MARKER`. Rust cannot
    // represent duplicate enum discriminants, so this parser is routed from the
    // existing 0xBADD opcode slot by payload shape in `WorldSession`.
    const OPCODE: ClientOpcodes = ClientOpcodes::SetLootSpecialization;

    fn read(pkt: &mut WorldPacket) -> Result<Self, PacketError> {
        Ok(Self {
            map_id: pkt.read_int32()?,
            difficulty_id: pkt.read_uint32()?,
            extend: pkt.read_bit()?,
        })
    }
}
