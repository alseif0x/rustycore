//! Classification packets.
//!
//! Separated from item.rs under #701.

use super::*;

/// Item class (major item category).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(i8)]
pub enum ItemClass {
    None = -1,
    Consumable = 0,
    Container = 1,
    Weapon = 2,
    Gem = 3,
    Armor = 4,
    Reagent = 5,
    Projectile = 6,
    TradeGoods = 7,
    ItemEnhancement = 8,
    Recipe = 9,
    Money = 10,
    Quiver = 11,
    Quest = 12,
    Key = 13,
    Permanent = 14,
    Miscellaneous = 15,
    Glyph = 16,
    BattlePets = 17,
    WowToken = 18,
    Profession = 19,
}

/// Consumable subclass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ItemSubClassConsumable {
    Consumable = 0,
    Potion = 1,
    Elixir = 2,
    Flask = 3,
    Scroll = 4,
    FoodDrink = 5,
    ItemEnhancement = 6,
    Bandage = 7,
    ConsumableOther = 8,
    VantusRune = 9,
}

/// Container subclass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ItemSubClassContainer {
    Container = 0,
    SoulContainer = 1,
    HerbContainer = 2,
    EnchantingContainer = 3,
    EngineeringContainer = 4,
    GemContainer = 5,
    MiningContainer = 6,
    LeatherworkingContainer = 7,
    InscriptionContainer = 8,
    TackleContainer = 9,
    CookingContainer = 10,
    ReagentContainer = 11,
}

/// Weapon subclass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ItemSubClassWeapon {
    Axe = 0,
    Axe2 = 1,
    Bow = 2,
    Gun = 3,
    Mace = 4,
    Mace2 = 5,
    Polearm = 6,
    Sword = 7,
    Sword2 = 8,
    Warglaives = 9,
    Staff = 10,
    Exotic = 11,
    Exotic2 = 12,
    Fist = 13,
    Miscellaneous = 14,
    Dagger = 15,
    Thrown = 16,
    Spear = 17,
    Crossbow = 18,
    Wand = 19,
    FishingPole = 20,
}

/// Gem subclass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ItemSubClassGem {
    Intellect = 0,
    Agility = 1,
    Strength = 2,
    Stamina = 3,
    Spirit = 4,
    CriticalStrike = 5,
    Mastery = 6,
    Haste = 7,
    Versatility = 8,
    Other = 9,
    MultipleStats = 10,
    ArtifactRelic = 11,
}

/// Armor subclass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ItemSubClassArmor {
    Miscellaneous = 0,
    Cloth = 1,
    Leather = 2,
    Mail = 3,
    Plate = 4,
    Cosmetic = 5,
    Shield = 6,
    Libram = 7,
    Idol = 8,
    Totem = 9,
    Sigil = 10,
    Relic = 11,
}

/// Reagent subclass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ItemSubClassReagent {
    Reagent = 0,
    Keystone = 1,
    ContextToken = 2,
}

/// Projectile subclass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ItemSubClassProjectile {
    Wand = 0,
    Bolt = 1,
    Arrow = 2,
    Bullet = 3,
    Thrown = 4,
}

/// Trade goods subclass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ItemSubClassTradeGoods {
    TradeGoods = 0,
    Parts = 1,
    Explosives = 2,
    Devices = 3,
    Jewelcrafting = 4,
    Cloth = 5,
    Leather = 6,
    MetalStone = 7,
    Meat = 8,
    Herb = 9,
    Elemental = 10,
    TradeGoodsOther = 11,
    Enchanting = 12,
    Material = 13,
    Enchantment = 14,
    WeaponEnchantment = 15,
    Inscription = 16,
    ExplosivesDevices = 17,
    OptionalReagent = 18,
    FinishingReagent = 19,
}

/// Item enhancement subclass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ItemSubclassItemEnhancement {
    Head = 0,
    Neck = 1,
    Shoulder = 2,
    Cloak = 3,
    Chest = 4,
    Wrist = 5,
    Hands = 6,
    Waist = 7,
    Legs = 8,
    Feet = 9,
    Finger = 10,
    Weapon = 11,
    TwoHandedWeapon = 12,
    ShieldOffHand = 13,
    Misc = 14,
}

/// Recipe subclass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ItemSubClassRecipe {
    Book = 0,
    LeatherworkingPattern = 1,
    TailoringPattern = 2,
    EngineeringSchematic = 3,
    Blacksmithing = 4,
    CookingRecipe = 5,
    AlchemyRecipe = 6,
    FirstAidManual = 7,
    EnchantingFormula = 8,
    FishingManual = 9,
    JewelcraftingRecipe = 10,
    InscriptionTechnique = 11,
}

/// Money subclass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ItemSubClassMoney {
    Money = 0,
}

/// Quiver subclass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ItemSubClassQuiver {
    Quiver0 = 0,
    Quiver1 = 1,
    Quiver = 2,
    AmmoPouch = 3,
}

/// Quest item subclass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ItemSubClassQuest {
    Quest = 0,
    Unk3 = 3,
    Unk8 = 8,
}

/// Key subclass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ItemSubClassKey {
    Key = 0,
    Lockpick = 1,
}

/// Permanent subclass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ItemSubClassPermanent {
    Permanent = 0,
}

/// Miscellaneous item subclass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ItemSubClassMisc {
    Junk = 0,
    Reagent = 1,
    CompanionPet = 2,
    Holiday = 3,
    Other = 4,
    Mount = 5,
    MountEquipment = 6,
}

/// Glyph subclass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ItemSubClassGlyph {
    Warrior = 1,
    Paladin = 2,
    Hunter = 3,
    Rogue = 4,
    Priest = 5,
    DeathKnight = 6,
    Shaman = 7,
    Mage = 8,
    Warlock = 9,
    Monk = 10,
    Druid = 11,
    DemonHunter = 12,
}

/// Battle pet subclass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ItemSubclassBattlePet {
    BattlePet = 0,
}

/// WoW token subclass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ItemSubclassWowToken {
    WowToken = 0,
}

/// Profession subclass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ItemSubclassProfession {
    Blacksmithing = 0,
    Leatherworking = 1,
    Alchemy = 2,
    Herbalism = 3,
    Cooking = 4,
    Mining = 5,
    Tailoring = 6,
    Engineering = 7,
    Enchanting = 8,
    Fishing = 9,
    Skinning = 10,
    Jewelcrafting = 11,
    Inscription = 12,
    Archaeology = 13,
}
