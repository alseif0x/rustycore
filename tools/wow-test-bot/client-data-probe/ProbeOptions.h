#ifndef RUSTYCORE_FOREVER_PROBE_OPTIONS_H
#define RUSTYCORE_FOREVER_PROBE_OPTIONS_H
#include <filesystem>
#include <stdexcept>
#include <string>

namespace ProbeOptions
{
struct Options
{
    std::filesystem::path tactKeyFile;
    bool availableAchievements = false;
    bool tactKeyTable = false;
    bool characterCustomizationTables = false;
    bool nameValidationTables = false;
    bool characterInitializationTables = false;
    bool availableInitialMap = false;
    bool characterBirthTables = false;
    bool availableBirthAbilities = false;
    bool characterItemTables = false;
    bool itemTableMetadataOnly = false;
    bool availableItemTables = false;
    bool itemTemplateTables = false;
    bool spellInfoTables = false;
    bool spellTableMetadataOnly = false;
    bool availableSpellInfoTables = false;
    bool spellValueGameTables = false;
};

// Parsing only. The acquisition owner supplies the existing private-file
// validator, retaining validation order before storage access or output writes.
inline Options Parse(int argc, char** argv,
    std::filesystem::path (*validateKeyFile)(std::filesystem::path const&))
{
    if (argc < 5 || argc > 23 || std::string(argv[1]) != "--ack-local-client-data")
        throw std::runtime_error("Usage: --ack-local-client-data <WoW storage root> <new private output directory> <esES|enUS> [--ack-public-tact-keys <private-file>] [--ack-available-achievements] [--ack-tact-key-table] [--ack-character-customization-tables] [--ack-name-validation-tables] [--ack-character-initialization-tables] [--ack-available-initial-map] [--ack-character-birth-tables] [--ack-available-birth-abilities] [--ack-character-item-tables] [--ack-item-template-tables] [--ack-item-table-metadata-only] [--ack-available-item-tables] [--ack-spell-info-tables] [--ack-spell-table-metadata-only] [--ack-available-spell-info-tables] [--ack-spell-value-game-tables]");
    Options result;
    bool tactKeyOptionSeen = false;
    for (int argument = 5; argument < argc;)
    {
        std::string option = argv[argument];
        if (option == "--ack-public-tact-keys")
        {
            if (tactKeyOptionSeen || argument + 1 >= argc)
                throw std::runtime_error("TACT key option requires one private file and may appear once");
            tactKeyOptionSeen = true;
            result.tactKeyFile = validateKeyFile(argv[argument + 1]);
            argument += 2;
        }
        else
        {
            bool* flag = nullptr;
            char const* duplicate = nullptr;
            if (option == "--ack-available-achievements")
            { flag = &result.availableAchievements; duplicate = "Available Achievement option may appear once"; }
            else if (option == "--ack-tact-key-table")
            { flag = &result.tactKeyTable; duplicate = "TACT key table option may appear once"; }
            else if (option == "--ack-character-customization-tables")
            { flag = &result.characterCustomizationTables; duplicate = "Character customization table option may appear once"; }
            else if (option == "--ack-name-validation-tables")
            { flag = &result.nameValidationTables; duplicate = "Name-validation table option may appear once"; }
            else if (option == "--ack-character-initialization-tables")
            { flag = &result.characterInitializationTables; duplicate = "Character initialization table option may appear once"; }
            else if (option == "--ack-available-initial-map")
            { flag = &result.availableInitialMap; duplicate = "Available initial Map option may appear once"; }
            else if (option == "--ack-character-birth-tables")
            { flag = &result.characterBirthTables; duplicate = "Character birth table option may appear once"; }
            else if (option == "--ack-available-birth-abilities")
            { flag = &result.availableBirthAbilities; duplicate = "Available birth-ability option may appear once"; }
            else if (option == "--ack-character-item-tables")
            { flag = &result.characterItemTables; duplicate = "Character item table option may appear once"; }
            else if (option == "--ack-item-table-metadata-only")
            { flag = &result.itemTableMetadataOnly; duplicate = "Item table metadata option may appear once"; }
            else if (option == "--ack-available-item-tables")
            { flag = &result.availableItemTables; duplicate = "Available item table option may appear once"; }
            else if (option == "--ack-item-template-tables")
            { flag = &result.itemTemplateTables; duplicate = "Item template table option may appear once"; }
            else if (option == "--ack-spell-info-tables")
            { flag = &result.spellInfoTables; duplicate = "SpellInfo table option may appear once"; }
            else if (option == "--ack-spell-value-game-tables")
            { flag = &result.spellValueGameTables; duplicate = "Spell value GameTable option may appear once"; }
            else if (option == "--ack-spell-table-metadata-only")
            { flag = &result.spellTableMetadataOnly; duplicate = "Spell table metadata option may appear once"; }
            else if (option == "--ack-available-spell-info-tables")
            { flag = &result.availableSpellInfoTables; duplicate = "Available SpellInfo table option may appear once"; }
            else throw std::runtime_error("Unknown optional client-data probe argument");
            if (*flag) throw std::runtime_error(duplicate);
            *flag = true;
            ++argument;
        }
    }
    if (result.availableInitialMap && !result.characterInitializationTables)
        throw std::runtime_error("Available initial Map requires initialization-table acknowledgement");
    if (result.availableBirthAbilities && !result.characterBirthTables)
        throw std::runtime_error("Available birth abilities require birth-table acknowledgement");
    if (result.itemTableMetadataOnly && !result.characterItemTables && !result.itemTemplateTables)
        throw std::runtime_error("Item metadata requires item-table acknowledgement");
    if (result.availableItemTables && !result.characterItemTables)
        throw std::runtime_error("Available items require item-table acknowledgement");
    if (result.availableItemTables && result.itemTableMetadataOnly)
        throw std::runtime_error("Available items and metadata-only are mutually exclusive");
    if (result.spellTableMetadataOnly && !result.spellInfoTables)
        throw std::runtime_error("Spell metadata requires SpellInfo-table acknowledgement");
    if (result.availableSpellInfoTables && !result.spellInfoTables)
        throw std::runtime_error("Available spells require SpellInfo-table acknowledgement");
    if (result.availableSpellInfoTables && result.spellTableMetadataOnly)
        throw std::runtime_error("Available spells and spell metadata-only are mutually exclusive");
    return result;
}
}
#endif
