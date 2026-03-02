use std::fmt;
use std::sync::atomic::{AtomicI64, Ordering};

/// High-type discriminator for ObjectGuid. Stored in bits [63:58] of the high qword.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum HighGuid {
    Null = 0,
    Uniq = 1,
    Player = 2,
    Item = 3,
    WorldTransaction = 4,
    StaticDoor = 5,
    Transport = 6,
    Conversation = 7,
    Creature = 8,
    Vehicle = 9,
    Pet = 10,
    GameObject = 11,
    DynamicObject = 12,
    AreaTrigger = 13,
    Corpse = 14,
    LootObject = 15,
    SceneObject = 16,
    Scenario = 17,
    AIGroup = 18,
    DynamicDoor = 19,
    ClientActor = 20,
    Vignette = 21,
    CallForHelp = 22,
    AIResource = 23,
    AILock = 24,
    AILockTicket = 25,
    ChatChannel = 26,
    Party = 27,
    Guild = 28,
    WowAccount = 29,
    BNetAccount = 30,
    GMTask = 31,
    MobileSession = 32,
    RaidGroup = 33,
    Spell = 34,
    Mail = 35,
    WebObj = 36,
    LFGObject = 37,
    LFGList = 38,
    UserRouter = 39,
    PVPQueueGroup = 40,
    UserClient = 41,
    PetBattle = 42,
    UniqUserClient = 43,
    BattlePet = 44,
    CommerceObj = 45,
    ClientSession = 46,
    Cast = 47,
    ClientConnection = 48,
    ClubFinder = 49,
    ToolsClient = 50,
    WorldLayer = 51,
    ArenaTeam = 52,
    LMMParty = 53,
    LMMLobby = 54,
    Count = 55,
}

impl HighGuid {
    pub fn from_u8(val: u8) -> Option<Self> {
        if val < Self::Count as u8 {
            // SAFETY: all values 0..55 are valid discriminants
            Some(unsafe { std::mem::transmute(val) })
        } else {
            None
        }
    }
}

/// TypeId for game objects — maps from HighGuid to the object type system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TypeId {
    Object = 0,
    Item = 1,
    Container = 2,
    AzeriteEmpoweredItem = 3,
    AzeriteItem = 4,
    Unit = 5,
    Player = 6,
    ActivePlayer = 7,
    GameObject = 8,
    DynamicObject = 9,
    Corpse = 10,
    AreaTrigger = 11,
    SceneObject = 12,
    Conversation = 13,
}

/// 128-bit packed object identifier, compatible with WoW 3.4.3 client.
///
/// Layout: `[high:i64][low:i64]`
///
/// High qword bit layout (varies by HighGuid type):
///   - Bits [63:58] = HighGuid type (6 bits)
///   - Bits [57:42] = Realm ID (13 bits) for realm-specific GUIDs
///   - Bits [41:29] = Map ID (13 bits) for map-specific GUIDs
///   - Bits [28:6]  = Entry ID (23 bits) for world objects
///   - Bits [5:0]   = SubType (6 bits) for cast GUIDs
///
/// Low qword bit layout:
///   - Bits [63:40] = Server ID (24 bits) for world objects
///   - Bits [39:0]  = Counter (40 bits)
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ObjectGuid {
    high: i64,
    low: i64,
}

impl ObjectGuid {
    pub const EMPTY: Self = Self { high: 0, low: 0 };

    #[inline]
    pub fn new(high: i64, low: i64) -> Self {
        Self { high, low }
    }

    // ── Accessors ──

    #[inline]
    pub fn high_value(&self) -> i64 {
        self.high
    }

    #[inline]
    pub fn low_value(&self) -> i64 {
        self.low
    }

    #[inline]
    pub fn high_type(&self) -> HighGuid {
        let raw = ((self.high >> 58) & 0x3F) as u8;
        HighGuid::from_u8(raw).unwrap_or(HighGuid::Null)
    }

    #[inline]
    pub fn sub_type(&self) -> u8 {
        (self.high & 0x3F) as u8
    }

    #[inline]
    pub fn realm_id(&self) -> u16 {
        ((self.high >> 42) & 0x1FFF) as u16
    }

    #[inline]
    pub fn server_id(&self) -> u32 {
        ((self.low >> 40) & 0x1FFF) as u32
    }

    #[inline]
    pub fn map_id(&self) -> u16 {
        ((self.high >> 29) & 0x1FFF) as u16
    }

    #[inline]
    pub fn entry(&self) -> u32 {
        ((self.high >> 6) & 0x7F_FFFF) as u32
    }

    #[inline]
    pub fn counter(&self) -> i64 {
        if self.high_type() == HighGuid::Transport {
            (self.high >> 38) & 0xF_FFFF
        } else {
            self.low & 0xFF_FFFF_FFFF
        }
    }

    #[inline]
    pub fn max_counter(high: HighGuid) -> i64 {
        if high == HighGuid::Transport {
            0xF_FFFF
        } else {
            0xFF_FFFF_FFFF
        }
    }

    /// Serialize to 16 raw bytes [low_bytes .. high_bytes].
    pub fn to_raw_bytes(&self) -> [u8; 16] {
        let mut buf = [0u8; 16];
        buf[..8].copy_from_slice(&self.low.to_le_bytes());
        buf[8..].copy_from_slice(&self.high.to_le_bytes());
        buf
    }

    /// Deserialize from 16 raw bytes [low_bytes .. high_bytes].
    pub fn from_raw_bytes(bytes: &[u8; 16]) -> Self {
        let low = i64::from_le_bytes(bytes[..8].try_into().unwrap());
        let high = i64::from_le_bytes(bytes[8..].try_into().unwrap());
        Self { high, low }
    }

    // ── Type checks ──

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.high == 0 && self.low == 0
    }

    #[inline]
    pub fn is_creature(&self) -> bool {
        self.high_type() == HighGuid::Creature
    }

    #[inline]
    pub fn is_pet(&self) -> bool {
        self.high_type() == HighGuid::Pet
    }

    #[inline]
    pub fn is_vehicle(&self) -> bool {
        self.high_type() == HighGuid::Vehicle
    }

    #[inline]
    pub fn is_creature_or_pet(&self) -> bool {
        self.is_creature() || self.is_pet()
    }

    #[inline]
    pub fn is_creature_or_vehicle(&self) -> bool {
        self.is_creature() || self.is_vehicle()
    }

    #[inline]
    pub fn is_any_type_creature(&self) -> bool {
        self.is_creature() || self.is_pet() || self.is_vehicle()
    }

    #[inline]
    pub fn is_player(&self) -> bool {
        !self.is_empty() && self.high_type() == HighGuid::Player
    }

    #[inline]
    pub fn is_unit(&self) -> bool {
        self.is_any_type_creature() || self.is_player()
    }

    #[inline]
    pub fn is_item(&self) -> bool {
        self.high_type() == HighGuid::Item
    }

    #[inline]
    pub fn is_game_object(&self) -> bool {
        self.high_type() == HighGuid::GameObject
    }

    #[inline]
    pub fn is_dynamic_object(&self) -> bool {
        self.high_type() == HighGuid::DynamicObject
    }

    #[inline]
    pub fn is_corpse(&self) -> bool {
        self.high_type() == HighGuid::Corpse
    }

    #[inline]
    pub fn is_area_trigger(&self) -> bool {
        self.high_type() == HighGuid::AreaTrigger
    }

    #[inline]
    pub fn is_mo_transport(&self) -> bool {
        self.high_type() == HighGuid::Transport
    }

    #[inline]
    pub fn is_any_type_game_object(&self) -> bool {
        self.is_game_object() || self.is_mo_transport()
    }

    #[inline]
    pub fn is_party(&self) -> bool {
        self.high_type() == HighGuid::Party
    }

    #[inline]
    pub fn is_guild(&self) -> bool {
        self.high_type() == HighGuid::Guild
    }

    #[inline]
    pub fn is_scene_object(&self) -> bool {
        self.high_type() == HighGuid::SceneObject
    }

    #[inline]
    pub fn is_conversation(&self) -> bool {
        self.high_type() == HighGuid::Conversation
    }

    #[inline]
    pub fn is_cast(&self) -> bool {
        self.high_type() == HighGuid::Cast
    }

    /// Map HighGuid to TypeId.
    pub fn type_id(&self) -> TypeId {
        Self::type_id_for(self.high_type())
    }

    pub fn type_id_for(high: HighGuid) -> TypeId {
        match high {
            HighGuid::Item => TypeId::Item,
            HighGuid::Creature | HighGuid::Pet | HighGuid::Vehicle => TypeId::Unit,
            HighGuid::Player => TypeId::Player,
            HighGuid::GameObject | HighGuid::Transport => TypeId::GameObject,
            HighGuid::DynamicObject => TypeId::DynamicObject,
            HighGuid::Corpse => TypeId::Corpse,
            HighGuid::AreaTrigger => TypeId::AreaTrigger,
            HighGuid::SceneObject => TypeId::SceneObject,
            HighGuid::Conversation => TypeId::Conversation,
            _ => TypeId::Object,
        }
    }

    pub fn has_entry(&self) -> bool {
        // In the C# code, HasEntry always returns true for all types
        true
    }

    pub fn is_map_specific(high: HighGuid) -> bool {
        matches!(
            high,
            HighGuid::Conversation
                | HighGuid::Creature
                | HighGuid::Vehicle
                | HighGuid::Pet
                | HighGuid::GameObject
                | HighGuid::DynamicObject
                | HighGuid::AreaTrigger
                | HighGuid::Corpse
                | HighGuid::LootObject
                | HighGuid::SceneObject
                | HighGuid::Scenario
                | HighGuid::AIGroup
                | HighGuid::DynamicDoor
                | HighGuid::Vignette
                | HighGuid::CallForHelp
                | HighGuid::AIResource
                | HighGuid::AILock
                | HighGuid::AILockTicket
        )
    }

    pub fn is_realm_specific(high: HighGuid) -> bool {
        matches!(
            high,
            HighGuid::Player
                | HighGuid::Item
                | HighGuid::ChatChannel
                | HighGuid::Transport
                | HighGuid::Guild
        )
    }

    pub fn is_global(high: HighGuid) -> bool {
        matches!(
            high,
            HighGuid::Uniq
                | HighGuid::Party
                | HighGuid::WowAccount
                | HighGuid::BNetAccount
                | HighGuid::GMTask
                | HighGuid::RaidGroup
                | HighGuid::Spell
                | HighGuid::Mail
                | HighGuid::UserRouter
                | HighGuid::PVPQueueGroup
                | HighGuid::UserClient
                | HighGuid::UniqUserClient
                | HighGuid::BattlePet
        )
    }

    // ── Factory methods ──

    pub fn create_null() -> Self {
        Self::EMPTY
    }

    pub fn create_uniq(id: i64) -> Self {
        Self::new((HighGuid::Uniq as i64) << 58, id)
    }

    pub fn create_player(realm_id: u16, db_id: i64) -> Self {
        Self::new(
            ((HighGuid::Player as i64) << 58) | ((realm_id as i64 & 0x1FFF) << 42),
            db_id,
        )
    }

    pub fn create_item(realm_id: u16, db_id: i64) -> Self {
        Self::new(
            ((HighGuid::Item as i64) << 58) | ((realm_id as i64 & 0x1FFF) << 42),
            db_id,
        )
    }

    pub fn create_world_object(
        guid_type: HighGuid,
        sub_type: u8,
        realm_id: u16,
        map_id: u16,
        server_id: u32,
        entry: u32,
        counter: i64,
    ) -> Self {
        Self::new(
            ((guid_type as i64) << 58)
                | ((realm_id as i64 & 0x1FFF) << 42)
                | ((map_id as i64 & 0x1FFF) << 29)
                | ((entry as i64 & 0x7F_FFFF) << 6)
                | (sub_type as i64 & 0x3F),
            ((server_id as i64 & 0xFF_FFFF) << 40) | (counter & 0xFF_FFFF_FFFF),
        )
    }

    pub fn create_transport(guid_type: HighGuid, counter: i64) -> Self {
        Self::new(((guid_type as i64) << 58) | (counter << 38), 0)
    }

    pub fn create_global(guid_type: HighGuid, db_id_high: i64, db_id: i64) -> Self {
        Self::new(
            ((guid_type as i64) << 58) | (db_id_high & 0x3FF_FFFF_FFFF_FFFF),
            db_id,
        )
    }

    pub fn create_guild(guid_type: HighGuid, realm_id: u16, db_id: i64) -> Self {
        Self::new(
            ((guid_type as i64) << 58) | ((realm_id as i64 & 0x1FFF) << 42),
            db_id,
        )
    }

    pub fn create_chat_channel(
        realm_id: u16,
        built_in: bool,
        trade: bool,
        zone_id: u16,
        faction_group_mask: u8,
        counter: i64,
    ) -> Self {
        Self::new(
            ((HighGuid::ChatChannel as i64) << 58)
                | ((realm_id as i64 & 0x1FFF) << 42)
                | ((built_in as i64) << 25)
                | ((trade as i64) << 24)
                | ((zone_id as i64 & 0x3FFF) << 10)
                | ((faction_group_mask as i64 & 0x3F) << 4),
            counter,
        )
    }

    pub fn create_client_actor(owner_type: u16, owner_id: u16, counter: u32) -> Self {
        Self::new(
            ((HighGuid::ClientActor as i64) << 58)
                | ((owner_type as i64 & 0x1FFF) << 42)
                | ((owner_id as i64 & 0xFF_FFFF) << 26),
            counter as i64,
        )
    }

    pub fn create_client(guid_type: HighGuid, realm_id: u16, arg1: i32, counter: i64) -> Self {
        Self::new(
            ((guid_type as i64) << 58)
                | ((realm_id as i64 & 0x1FFF) << 42)
                | ((arg1 as i64 & 0xFFFF_FFFF) << 10),
            counter,
        )
    }

    /// Create a Party/Group GUID from a counter value.
    pub fn create_group(counter: u64) -> Self {
        Self::create_global(HighGuid::Party, 0, counter as i64)
    }
}

impl PartialOrd for ObjectGuid {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ObjectGuid {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.high
            .cmp(&other.high)
            .then(self.low.cmp(&other.low))
    }
}

impl fmt::Debug for ObjectGuid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GUID[{:?} entry:{} counter:{}]",
            self.high_type(),
            self.entry(),
            self.counter()
        )
    }
}

impl fmt::Display for ObjectGuid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GUID Full: 0x{:X}, Type: {:?}",
            (self.high as u128) << 64 | self.low as u128,
            self.high_type()
        )?;
        if self.has_entry() {
            if self.is_pet() {
                write!(f, " Pet number: {} ", self.entry())?;
            } else {
                write!(f, " Entry: {} ", self.entry())?;
            }
        }
        write!(f, " Low: {}", self.counter())
    }
}

/// Thread-safe GUID counter generator.
pub struct ObjectGuidGenerator {
    next_guid: AtomicI64,
    high_guid: HighGuid,
}

impl ObjectGuidGenerator {
    pub fn new(high_guid: HighGuid, start: i64) -> Self {
        Self {
            next_guid: AtomicI64::new(start),
            high_guid,
        }
    }

    pub fn generate(&self) -> i64 {
        let val = self.next_guid.fetch_add(1, Ordering::Relaxed);
        assert!(
            val < ObjectGuid::max_counter(self.high_guid) - 1,
            "{:?} guid overflow! Cannot continue.",
            self.high_guid
        );
        val
    }

    pub fn set(&self, val: i64) {
        self.next_guid.store(val, Ordering::Relaxed);
    }

    pub fn next_after_max_used(&self) -> i64 {
        self.next_guid.load(Ordering::Relaxed)
    }

    pub fn high_guid(&self) -> HighGuid {
        self.high_guid
    }
}

// ── Typed GUID newtypes ──

/// A GUID that is guaranteed to refer to a Player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlayerGuid(ObjectGuid);

impl PlayerGuid {
    pub fn new(guid: ObjectGuid) -> Option<Self> {
        if guid.is_player() {
            Some(Self(guid))
        } else {
            None
        }
    }

    pub fn inner(&self) -> ObjectGuid {
        self.0
    }

    pub fn counter(&self) -> i64 {
        self.0.counter()
    }

    pub fn realm_id(&self) -> u16 {
        self.0.realm_id()
    }
}

/// A GUID that is guaranteed to refer to a Creature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CreatureGuid(ObjectGuid);

impl CreatureGuid {
    pub fn new(guid: ObjectGuid) -> Option<Self> {
        if guid.is_creature() {
            Some(Self(guid))
        } else {
            None
        }
    }

    pub fn inner(&self) -> ObjectGuid {
        self.0
    }

    pub fn entry(&self) -> u32 {
        self.0.entry()
    }

    pub fn counter(&self) -> i64 {
        self.0.counter()
    }
}

impl ObjectGuid {
    pub fn as_player(&self) -> Option<PlayerGuid> {
        PlayerGuid::new(*self)
    }

    pub fn as_creature(&self) -> Option<CreatureGuid> {
        CreatureGuid::new(*self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_guid() {
        let guid = ObjectGuid::EMPTY;
        assert!(guid.is_empty());
        assert_eq!(guid.high_type(), HighGuid::Null);
        assert_eq!(guid.counter(), 0);
    }

    #[test]
    fn test_create_player() {
        let guid = ObjectGuid::create_player(1, 42);
        assert!(guid.is_player());
        assert!(!guid.is_empty());
        assert_eq!(guid.high_type(), HighGuid::Player);
        assert_eq!(guid.realm_id(), 1);
        assert_eq!(guid.counter(), 42);
        assert_eq!(guid.type_id(), TypeId::Player);
    }

    #[test]
    fn test_create_creature() {
        let guid = ObjectGuid::create_world_object(
            HighGuid::Creature,
            0,   // sub_type
            1,   // realm_id
            530, // map_id (Outland)
            0,   // server_id
            1234, // entry
            5678, // counter
        );
        assert!(guid.is_creature());
        assert_eq!(guid.high_type(), HighGuid::Creature);
        assert_eq!(guid.map_id(), 530);
        assert_eq!(guid.entry(), 1234);
        assert_eq!(guid.counter(), 5678);
        assert_eq!(guid.type_id(), TypeId::Unit);
    }

    #[test]
    fn test_create_item() {
        let guid = ObjectGuid::create_item(1, 99999);
        assert!(guid.is_item());
        assert_eq!(guid.counter(), 99999);
        assert_eq!(guid.type_id(), TypeId::Item);
    }

    #[test]
    fn test_create_uniq() {
        let guid = ObjectGuid::create_uniq(10);
        assert_eq!(guid.high_type(), HighGuid::Uniq);
        assert_eq!(guid.counter(), 10);
    }

    #[test]
    fn test_create_transport() {
        let guid = ObjectGuid::create_transport(HighGuid::Transport, 100);
        assert!(guid.is_mo_transport());
        assert!(guid.is_any_type_game_object());
        assert_eq!(guid.counter(), 100);
    }

    #[test]
    fn test_create_global() {
        let guid = ObjectGuid::create_global(HighGuid::Party, 0, 12345);
        assert!(guid.is_party());
        assert_eq!(guid.counter(), 12345);
    }

    #[test]
    fn test_create_guild() {
        let guid = ObjectGuid::create_guild(HighGuid::Guild, 1, 42);
        assert!(guid.is_guild());
        assert_eq!(guid.realm_id(), 1);
        assert_eq!(guid.counter(), 42);
    }

    #[test]
    fn test_raw_bytes_roundtrip() {
        let original = ObjectGuid::create_world_object(
            HighGuid::Creature, 0, 1, 571, 0, 9999, 123456,
        );
        let bytes = original.to_raw_bytes();
        let restored = ObjectGuid::from_raw_bytes(&bytes);
        assert_eq!(original, restored);
    }

    #[test]
    fn test_guid_ordering() {
        let a = ObjectGuid::create_player(1, 1);
        let b = ObjectGuid::create_player(1, 2);
        assert!(a < b);
    }

    #[test]
    fn test_guid_generator() {
        let generator = ObjectGuidGenerator::new(HighGuid::Creature, 1);
        assert_eq!(generator.generate(), 1);
        assert_eq!(generator.generate(), 2);
        assert_eq!(generator.generate(), 3);
        assert_eq!(generator.next_after_max_used(), 4);
    }

    #[test]
    fn test_typed_guid_player() {
        let guid = ObjectGuid::create_player(1, 42);
        let player = guid.as_player();
        assert!(player.is_some());
        assert_eq!(player.unwrap().counter(), 42);

        let creature_guid = ObjectGuid::create_world_object(
            HighGuid::Creature, 0, 0, 0, 0, 1, 1,
        );
        assert!(creature_guid.as_player().is_none());
    }

    #[test]
    fn test_typed_guid_creature() {
        let guid = ObjectGuid::create_world_object(
            HighGuid::Creature, 0, 0, 0, 0, 100, 50,
        );
        let creature = guid.as_creature();
        assert!(creature.is_some());
        assert_eq!(creature.unwrap().entry(), 100);

        let player_guid = ObjectGuid::create_player(1, 42);
        assert!(player_guid.as_creature().is_none());
    }

    #[test]
    fn test_map_specific() {
        assert!(ObjectGuid::is_map_specific(HighGuid::Creature));
        assert!(ObjectGuid::is_map_specific(HighGuid::GameObject));
        assert!(!ObjectGuid::is_map_specific(HighGuid::Player));
        assert!(!ObjectGuid::is_map_specific(HighGuid::Item));
    }

    #[test]
    fn test_realm_specific() {
        assert!(ObjectGuid::is_realm_specific(HighGuid::Player));
        assert!(ObjectGuid::is_realm_specific(HighGuid::Item));
        assert!(!ObjectGuid::is_realm_specific(HighGuid::Creature));
    }

    #[test]
    fn test_is_global() {
        assert!(ObjectGuid::is_global(HighGuid::Party));
        assert!(ObjectGuid::is_global(HighGuid::BattlePet));
        assert!(!ObjectGuid::is_global(HighGuid::Player));
    }
}
