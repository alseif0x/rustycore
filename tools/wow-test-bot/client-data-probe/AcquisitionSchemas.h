#ifndef RUSTYCORE_FOREVER_ACQUISITION_SCHEMAS_H
#define RUSTYCORE_FOREVER_ACQUISITION_SCHEMAS_H

#include <CascLib.h>
#include <array>

namespace AcquisitionSchemas
{
struct Db2Schema
{
    char const* fileName;
    DWORD fileDataId;
    DWORD layoutHash;
    DWORD metaFieldCount;
    DWORD fileFieldCount;
    int indexField;
    int parentIndexField;
};

// DB2Metadata.h at 02245dcd. The table hash is deliberately not copied here:
// DB2Meta carries the target layout/field contract, while DB2Header::TableHash
// is reported from the acquired file. Physical fields and section ID-table
// rules are checked against this per-table metadata by the acquisition owner.
constexpr std::array<Db2Schema, 6> CharacterCustomizationSchemas = {{
    { "ChrModel.db2", 3384313, 0x03FAB755, 17, 17, 2, 4 },
    { "ChrCustomizationOption.db2", 3384247, 0xDCC2A86E, 13, 13, 1, 4 },
    { "ChrCustomizationChoice.db2", 3450554, 0x9559C358, 11, 11, 1, 2 },
    { "ChrCustomizationReq.db2", 3450453, 0xCA154412, 9, 9, -1, -1 },
    { "ChrRaceXChrModel.db2", 3490304, 0xA203BC29, 4, 4, -1, 0 },
    { "ChrCustomizationReqChoice.db2", 3580359, 0xF925BC6F, 2, 1, -1, 1 },
}};

// Separate opt-in: ObjectMgr::isValidString / DB2Manager::ValidateName.
constexpr std::array<Db2Schema, 4> NameValidationSchemas = {{
    { "NamesProfanity.db2", 1117086, 0xF227E638, 2, 2, -1, -1 },
    { "NamesReserved.db2", 1117085, 0x2B2D5D97, 1, 1, -1, -1 },
    { "NamesReservedLocale.db2", 1117087, 0x7B9823D4, 2, 2, -1, -1 },
    { "Cfg_Categories.db2", 1068162, 0x8710BE94, 6, 6, -1, -1 },
}};

// Maps/powers/specs/movie presence, not a playable Player or terrain data.
constexpr Db2Schema InitialMapSchema = { "Map.db2", 1349477, 0xD43AFAC3, 26, 26, -1, -1 };
constexpr std::array<Db2Schema, 5> CharacterInitializationSchemas = {{
    { "PowerType.db2", 1266022, 0x14BBEEA1, 13, 13, 2, -1 },
    { "ChrSpecialization.db2", 1343390, 0xDAB4CA4B, 13, 13, 3, 4 },
    { "ChrClassesXPowerTypes.db2", 1121420, 0x70DA1F8C, 2, 1, -1, 1 },
    { "Movie.db2", 1332556, 0xF53888FA, 6, 6, -1, -1 },
    InitialMapSchema,
}};

// DB2Metadata.h::{SkillLine,SkillRaceClassInfo,SkillLineAbility,
// CharacterLoadout,CharacterLoadoutItem} at 02245dcd. ObjectMgr.cpp:3968-4125
// joins these with effective ItemTemplate/SpellInfo and SQL skill tiers.
// Acquisition alone is not skill learning, spell granting or item placement.
constexpr std::array<Db2Schema, 5> CharacterBirthSchemas = {{
    { "SkillLine.db2", 1240935, 0x6763217C, 15, 15, 5, -1 },
    { "SkillRaceClassInfo.db2", 1240406, 0x24277B48, 7, 7, -1, 0 },
    { "SkillLineAbility.db2", 1266278, 0x224F7EA0, 18, 18, 2, 3 },
    { "CharacterLoadout.db2", 1344281, 0x713CE8BB, 5, 5, -1, -1 },
    { "CharacterLoadoutItem.db2", 1302846, 0x0C7A1862, 2, 2, -1, 0 },
}};

// DB2Metadata.h at 02245dcd; complete records only, no available-item mode.
// ItemSparse is the target 68-field schema, not the older 64-field layout.
constexpr std::array<Db2Schema, 4> CharacterItemSchemas = {{
    { "Item.db2", 841626, 0x9A2A4834, 16, 16, -1, -1 },
    { "ItemSparse.db2", 1572924, 0x6FCC3191, 68, 68, -1, -1 },
    { "ItemEffect.db2", 969941, 0x4CA77678, 9, 9, -1, -1 },
    { "ItemXItemEffect.db2", 3177687, 0x96F083AD, 2, 1, -1, 1 },
}};

// ObjectMgr::LoadItemTemplates specialization/relic inputs, 02245dcd.
// ItemSpec's parent is in-record; Override's item is an extra uint32 parent.
constexpr std::array<Db2Schema, 3> ItemTemplateSchemas = {{
    { "ItemSpec.db2", 1135120, 0x83F3D113, 6, 6, -1, 2 },
    { "ItemSpecOverride.db2", 1134576, 0xB292998C, 2, 1, -1, 1 },
    { "GemProperties.db2", 1343604, 0x86487AD2, 2, 2, -1, -1 },
}};
}
#endif
