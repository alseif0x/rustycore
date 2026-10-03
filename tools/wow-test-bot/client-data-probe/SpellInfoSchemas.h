#ifndef RUSTYCORE_FOREVER_SPELL_INFO_SCHEMAS_H
#define RUSTYCORE_FOREVER_SPELL_INFO_SCHEMAS_H

#include "AcquisitionSchemas.h"

namespace AcquisitionSchemas
{
// DB2Metadata.h at 02245dcd245e7433e524577656177723d3e4992e.
// SpellMgr.cpp::LoadSpellInfoStore / SpellInfo.cpp constructors and the
// related learn/form/category stores. This is an acquisition contract, not
// complete SpellInfo, SQL spell data, learning or casting implementation.
// Native table hashes are observed from 70170 headers, never inferred here.
constexpr std::array<Db2Schema, 49> SpellInfoSchemas = {{
    { "SpellName.db2", 1990283, 0x782EE721, 1, 1, -1, -1 },
    { "SpellEffect.db2", 1140088, 0x5362E3D4, 30, 29, -1, 29 },
    { "SpellMisc.db2", 1003144, 0x434B3607, 17, 16, -1, 16 },
    { "SpellAuraOptions.db2", 1139952, 0x95F4D5E1, 8, 7, -1, 7 },
    { "SpellAuraRestrictions.db2", 981566, 0xE7F2E213, 14, 13, -1, 13 },
    { "SpellCastingRequirements.db2", 1002166, 0x02074290, 7, 7, -1, -1 },
    { "SpellCategories.db2", 1139939, 0x679EF94C, 10, 9, -1, 9 },
    { "SpellClassOptions.db2", 979663, 0x1F8E4FD7, 4, 4, -1, -1 },
    { "SpellCooldowns.db2", 1139924, 0xDC945B8C, 6, 5, -1, 5 },
    { "SpellEmpower.db2", 4507381, 0x87C43ED2, 3, 3, 0, -1 },
    { "SpellEmpowerStage.db2", 4871072, 0x69BA286D, 3, 2, -1, 2 },
    { "SpellEquippedItems.db2", 1140011, 0xD9177916, 4, 4, -1, -1 },
    { "SpellInterrupts.db2", 1139906, 0x6FFC0306, 5, 4, -1, 4 },
    { "SpellLabel.db2", 1347275, 0x2B9F0138, 2, 1, -1, 1 },
    { "SpellLevels.db2", 1140079, 0x7EB86FDC, 6, 5, -1, 5 },
    { "SpellPower.db2", 982806, 0x61AD223F, 15, 14, 0, 14 },
    { "SpellPowerDifficulty.db2", 982804, 0xB80F1651, 2, 2, -1, -1 },
    { "SpellReagents.db2", 841946, 0xFF40D202, 5, 5, -1, -1 },
    { "SpellReagentsCurrency.db2", 1135239, 0x0ED4741A, 5, 5, -1, 0 },
    { "SpellScaling.db2", 1139940, 0x9FC07797, 3, 3, -1, -1 },
    { "SpellShapeshift.db2", 1139929, 0xE9111399, 4, 4, -1, -1 },
    { "SpellTargetRestrictions.db2", 1139993, 0x1EAB753E, 8, 7, -1, 7 },
    { "SpellTotems.db2", 1002162, 0x03B23619, 3, 3, -1, -1 },
    { "SpellXSpellVisual.db2", 1101657, 0x7994A890, 13, 12, 0, 12 },
    { "Difficulty.db2", 1352127, 0xB810C351, 14, 14, -1, -1 },
    { "SpellCastTimes.db2", 1134089, 0x75B6BD3A, 2, 2, -1, -1 },
    { "SpellDuration.db2", 1137828, 0xA931BD2B, 3, 3, -1, -1 },
    { "SpellRange.db2", 1146820, 0xADA13705, 5, 5, -1, -1 },
    { "SpellRadius.db2", 1134584, 0xF9A913EE, 4, 4, -1, -1 },
    { "SpellProcsPerMinute.db2", 1133526, 0x2227014B, 2, 2, -1, -1 },
    { "SpellProcsPerMinuteMod.db2", 1133525, 0xA89F22A1, 5, 4, -1, 4 },
    { "SpellLearnSpell.db2", 1001907, 0x6B9C3AC9, 3, 3, -1, 0 },
    { "SpellShapeshiftForm.db2", 1280618, 0xEE25A6A2, 10, 10, -1, -1 },
    { "SummonProperties.db2", 1345276, 0xA4CA5ECF, 5, 5, -1, -1 },
    { "BattlePetSpecies.db2", 841622, 0x589BE282, 12, 12, 2, -1 },
    { "SpellCategory.db2", 1280619, 0xB79B78E1, 6, 6, -1, -1 },
    // LoadSpellInfoCustomAttributes dependencies. Existing prefix indices
    // stay unchanged; these added stores remain strict until real evidence.
    { "Talent.db2", 1369062, 0x147B0045, 14, 14, -1, -1 },
    { "SpellItemEnchantment.db2", 1362771, 0x952B72B2, 24, 24, -1, -1 },
    { "SpellVisual.db2", 897952, 0x4B85C90F, 17, 17, -1, -1 },
    { "SpellVisualMissile.db2", 897954, 0xEC765EB2, 23, 22, 2, 22 },
    { "SpellVisualEffectName.db2", 897948, 0x2245CEE6, 16, 16, -1, -1 },
    { "LiquidType.db2", 1371380, 0xD1ECEEC9, 21, 21, -1, -1 },
    { "ExpectedStat.db2", 1937326, 0x0FD90F9C, 12, 11, -1, 11 },
    { "ExpectedStatMod.db2", 1969773, 0x8C41CCCE, 9, 9, -1, -1 },
    { "ContentTuning.db2", 1962930, 0xA3E13004, 19, 19, 0, -1 },
    { "ContentTuningXExpected.db2", 2976765, 0x897A4313, 4, 3, -1, 3 },
    { "RandPropPoints.db2", 1310245, 0x4FD22743, 10, 10, -1, -1 },
    { "MythicPlusSeason.db2", 2400282, 0xDC94262F, 5, 5, 0, -1 },
    { "UnitCondition.db2", 1120959, 0x215FAF83, 4, 4, -1, -1 },
}};
}
#endif
