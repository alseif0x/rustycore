//! Results packets.
//!
//! Separated from item.rs under #701.

use super::*;

/// Buy bank slot result codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum BuyBankSlotResult {
    FailedTooMany = 0,
    InsufficientFunds = 1,
    NotBanker = 2,
    OK = 3,
}

bitflags! {
    /// Custom item flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ItemFlagsCustom: u32 {
        const UNUSED                = 0x0001;
        const IGNORE_QUEST_STATUS   = 0x0002;
        const FOLLOW_LOOT_RULES     = 0x0004;
    }
}

/// Inventory operation result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum InventoryResult {
    Ok = 0,
    CantEquipLevelI = 1,
    CantEquipSkill = 2,
    WrongSlot = 3,
    BagFull = 4,
    BagInBag = 5,
    TradeEquippedBag = 6,
    AmmoOnly = 7,
    ProficiencyNeeded = 8,
    NoSlotAvailable = 9,
    CantEquipEver = 10,
    CantEquipEver2 = 11,
    NoSlotAvailable2 = 12,
    Equipped2handed = 13,
    TwoHandSkillNotFound = 14,
    WrongBagType = 15,
    WrongBagType2 = 16,
    ItemMaxCount = 17,
    NoSlotAvailable3 = 18,
    CantStack = 19,
    NotEquippable = 20,
    CantSwap = 21,
    SlotEmpty = 22,
    ItemNotFound = 23,
    DropBoundItem = 24,
    OutOfRange = 25,
    TooFewToSplit = 26,
    SplitFailed = 27,
    SpellFailedReagentsGeneric = 28,
    CantTradeGold = 29,
    NotEnoughMoney = 30,
    NotABag = 31,
    DestroyNonemptyBag = 32,
    NotOwner = 33,
    OnlyOneQuiver = 34,
    NoBankSlot = 35,
    NoBankHere = 36,
    ItemLocked = 37,
    GenericStunned = 38,
    PlayerDead = 39,
    ClientLockedOut = 40,
    InternalBagError = 41,
    OnlyOneBolt = 42,
    OnlyOneAmmo = 43,
    CantWrapStackable = 44,
    CantWrapEquipped = 45,
    CantWrapWrapped = 46,
    CantWrapBound = 47,
    CantWrapUnique = 48,
    CantWrapBags = 49,
    LootGone = 50,
    InvFull = 51,
    BankFull = 52,
    VendorSoldOut = 53,
    BagFull2 = 54,
    ItemNotFound2 = 55,
    CantStack2 = 56,
    BagFull3 = 57,
    VendorSoldOut2 = 58,
    ObjectIsBusy = 59,
    CantBeDisenchanted = 60,
    NotInCombat = 61,
    NotWhileDisarmed = 62,
    BagFull4 = 63,
    CantEquipRank = 64,
    CantEquipReputation = 65,
    TooManySpecialBags = 66,
    LootCantLootThatNow = 67,
    ItemUniqueEquippable = 68,
    VendorMissingTurnins = 69,
    NotEnoughHonorPoints = 70,
    NotEnoughArenaPoints = 71,
    ItemMaxCountSocketed = 72,
    MailBoundItem = 73,
    InternalBagError2 = 74,
    BagFull5 = 75,
    ItemMaxCountEquippedSocketed = 76,
    ItemUniqueEquippableSocketed = 77,
    TooMuchGold = 78,
    NotDuringArenaMatch = 79,
    TradeBoundItem = 80,
    CantEquipRating = 81,
    EventAutoequipBindConfirm = 82,
    NotSameAccount = 83,
    EquipNone3 = 84,
    ItemMaxLimitCategoryCountExceededIs = 85,
    ItemMaxLimitCategorySocketedExceededIs = 86,
    ScalingStatItemLevelExceeded = 87,
    PurchaseLevelTooLow = 88,
    CantEquipNeedTalent = 89,
    ItemMaxLimitCategoryEquippedExceededIs = 90,
    ShapeshiftFormCannotEquip = 91,
    ItemInventoryFullSatchel = 92,
    ScalingStatItemLevelTooLow = 93,
    CantBuyQuantity = 94,
    ItemIsBattlePayLocked = 95,
    ReagentBankFull = 96,
    ReagentBankLocked = 97,
    WrongBagType3 = 98,
    CantUseItem = 99,
    CantBeObliterated = 100,
    GuildBankConjuredItem = 101,
    BagFull6 = 102,
    BagFull7 = 103,
    CantBeScrapped = 104,
    BagFull8 = 105,
    NotInPetBattle = 106,
    BagFull9 = 107,
    CantDoThatRightNow = 108,
    CantDoThatRightNow2 = 109,
    NotInNPE = 110,
    ItemCooldown = 111,
    NotInRatedBattleground = 112,
    EquipableSpellsSlotsFull = 113,
    CantBeRecrafted = 114,
    ReagentBagWrongSlot = 115,
    SlotOnlyReagentBag = 116,
    ReagentBagItemType = 117,
    CantBulkSellItemWithRefund = 118,
}

/// Buy result codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u8)]
pub enum BuyResult {
    CantFindItem = 0,
    ItemAlreadySold = 1,
    NotEnoughtMoney = 2,
    SellerDontLikeYou = 4,
    DistanceTooFar = 5,
    ItemSoldOut = 7,
    CantCarryMore = 8,
    RankRequire = 11,
    ReputationRequire = 12,
}

/// Sell result codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum SellResult {
    CantFindItem = 1,
    CantSellItem = 2,
    CantFindVendor = 3,
    YouDontOwnThatItem = 4,
    Unk = 5,
    OnlyEmptyBag = 6,
    CantSellToThisMerchant = 7,
    MustRepairDurability = 8,
    VendorRefuseScappableAzerite = 9,
    InternalBagError = 10,
}

/// Item vendor type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u8)]
pub enum ItemVendorType {
    None = 0,
    Item = 1,
    Currency = 2,
    Spell = 3,
    MawPower = 4,
}

/// Currency types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum CurrencyTypes {
    JusticePoints = 395,
    ValorPoints = 396,
    ApexisCrystals = 823,
    Azerite = 1553,
    AncientMana = 1155,
}
