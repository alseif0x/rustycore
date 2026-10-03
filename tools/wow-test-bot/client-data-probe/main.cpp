// RustyCore operator-only acquisition of base build-70170 DB2 tables,
// with explicit opt-ins for TactKey.db2, character-customization data and
// name-validation and initial-character data.
// CascLib comes from the hash-pinned reference. No account or network access.
#include <CascLib.h>
#include "AcquisitionSchemas.h"
#include "BirthAbilityPrefix.h"
#include "GameTableOracle.h"
#include "ProbeOptions.h"
#include "ItemPrefixes.h"
#include "SpellInfoSchemas.h"
#include "SpellPrefixes.h"
#include <array>
#include <cerrno>
#include <cstring>
#include <filesystem>
#include <iostream>
#include <stdexcept>
#include <string>
#include <vector>
#include <fcntl.h>
#include <unistd.h>

namespace fs = std::filesystem;
namespace
{
constexpr DWORD Build = 70170;
constexpr DWORD TactKeyFileDataId = 1302850;
constexpr DWORD TactKeyTableHash = 0xDF2F53CF;
constexpr DWORD TactKeyLayoutHash = 0xCBA490FC;
constexpr DWORD TactKeyFieldCount = 1;
constexpr DWORD TactKeyRecordSize = 16;
constexpr unsigned short Db2SparseFlag = 0x0001;
constexpr unsigned short Db2ExternalIdListFlag = 0x0004;
constexpr DWORD AchievementFileDataId = 1260179;
constexpr DWORD AchievementTableHash = 0xD2EE2CA7;
constexpr DWORD AchievementLayoutHash = 0x6FC5281B;
constexpr DWORD AchievementRecordCount = 443;
constexpr DWORD AchievementFieldCount = 19;
constexpr DWORD AchievementAvailableRecords = 434;
constexpr DWORD AchievementUnavailableRecords = 9;
constexpr DWORD AchievementSectionCount = 2;
constexpr DWORD AchievementFirstSectionOffset = 976;
constexpr DWORD AchievementFirstSectionStringSize = 24317;
constexpr DWORD AchievementSecondSectionOffset = 40929;
constexpr DWORD AchievementSecondSectionStringSize = 514;
constexpr DWORD Db2HeaderSize = 204;
constexpr DWORD Db2SectionHeaderSize = 40;
constexpr ULONGLONG MaxBytes = 4 * 1024 * 1024;
constexpr ULONGLONG MaxTactKeyFileBytes = 10 * 1024 * 1024;

using AcquisitionSchemas::Db2Schema;
using AcquisitionSchemas::CharacterCustomizationSchemas;
using AcquisitionSchemas::NameValidationSchemas;
using AcquisitionSchemas::InitialMapSchema;
using AcquisitionSchemas::CharacterInitializationSchemas;
using AcquisitionSchemas::CharacterBirthSchemas;
using AcquisitionSchemas::CharacterItemSchemas;
using AcquisitionSchemas::ItemTemplateSchemas;
using AcquisitionSchemas::SpellInfoSchemas;

struct GameTableSchema
{
    char const* fileName;
    DWORD fileDataId;
    size_t valueColumns;
};

// map_extractor/System.cpp::ExtractGameTables and GameTables.h, 02245dcd.
// The first text column is ignored by the source loader; rows are indexed by
// their physical order, not by that label. Acquisition preserves all bytes.
constexpr std::array<GameTableSchema, 3> InitialGameTables = {{
    { "BaseMp.txt", 1391664, 15 },
    { "HpPerSta.txt", 1391642, 1 },
    { "xp.txt", 1391661, 5 },
}};
constexpr std::array<GameTableSchema, 3> SpellValueGameTables = {{
    { "SpellScaling.txt", 1391660, 24 },
    { "CombatRatingsMultByILvl.txt", 1391670, 4 },
    { "StaminaMultByILvl.txt", 1980632, 4 },
}};

// Fresh 70170/esES local observation: 71 plaintext Map rows and eight rows in
// three unavailable sections. An explicit prefix is not a complete Map.db2.
constexpr DWORD MapAvailableBytes = 9574;
void ValidateAvailableMap(std::vector<unsigned char> const& data);
struct Storage
{
    HANDLE value = nullptr;
    ~Storage() { if (value) CascCloseStorage(value); }
};
struct File
{
    HANDLE value = nullptr;
    ~File() { if (value) CascCloseFile(value); }
};
struct Descriptor
{
    int value;
    ~Descriptor() { if (value >= 0) close(value); }
};

bool WINAPI LocalOnly(void*, CASC_PROGRESS_MSG message, LPCSTR, DWORD, DWORD)
{
    return message == CascProgressDownloadingFile ||
        message == CascProgressDownloadingArchiveIndexes;
}

[[noreturn]] void CascFailure(char const* operation)
{
    // No storage key, decrypted content, filename-list or credential dump.
    throw std::runtime_error(std::string(operation) +
        " failed; CASC error=" + std::to_string(GetCascError()));
}

void SaveNew(fs::path const& path, std::vector<unsigned char> const& data)
{
    Descriptor file{open(path.c_str(), O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW, 0600)};
    if (file.value < 0) throw std::runtime_error("Cannot create a new private output file");
    size_t written = 0;
    while (written < data.size())
    {
        ssize_t count = write(file.value, data.data() + written, data.size() - written);
        if (count < 0 && errno == EINTR) continue;
        if (count <= 0) throw std::runtime_error("Private output write failed");
        written += static_cast<size_t>(count);
    }
}

DWORD HeaderWord(std::vector<unsigned char> const& data, size_t offset)
{
    if (offset > data.size() || data.size() - offset < 4)
        throw std::runtime_error("Truncated DB2 header word");
    return DWORD(data[offset]) | (DWORD(data[offset + 1]) << 8) |
        (DWORD(data[offset + 2]) << 16) | (DWORD(data[offset + 3]) << 24);
}

unsigned short HeaderHalfWord(std::vector<unsigned char> const& data, size_t offset)
{
    if (offset > data.size() || data.size() - offset < 2)
        throw std::runtime_error("Truncated DB2 header half-word");
    return static_cast<unsigned short>(data[offset]) | (static_cast<unsigned short>(data[offset + 1]) << 8);
}

ULONGLONG HeaderLong(std::vector<unsigned char> const& data, size_t offset)
{
    return ULONGLONG(HeaderWord(data, offset)) | (ULONGLONG(HeaderWord(data, offset + 4)) << 32);
}

void ValidateHeader(std::vector<unsigned char> const& data)
{
    // DB2FileLoader.h::DB2Header and DB2FileLoader::LoadHeaders, 02245dcd.
    // WDC5 adds Version + Schema[128]; WDC4 offsets are not interchangeable.
    if (data.size() < Db2HeaderSize || HeaderWord(data, 0) != 0x35434457 || HeaderWord(data, 4) != 5)
        throw std::runtime_error("Expected complete WDC5/version-5 header");
    if (HeaderWord(data, 184) > 1)
        throw std::runtime_error("Unsupported DB2 parent lookup count");
}

void SetHeaderWord(std::vector<unsigned char>& data, size_t offset, DWORD value)
{
    if (offset > data.size() || data.size() - offset < 4)
        throw std::runtime_error("Synthetic DB2 header write outside buffer");
    data[offset] = static_cast<unsigned char>(value);
    data[offset + 1] = static_cast<unsigned char>(value >> 8);
    data[offset + 2] = static_cast<unsigned char>(value >> 16);
    data[offset + 3] = static_cast<unsigned char>(value >> 24);
}

void SetHeaderHalfWord(std::vector<unsigned char>& data, size_t offset, unsigned short value)
{
    if (offset > data.size() || data.size() - offset < 2)
        throw std::runtime_error("Synthetic DB2 header half-word write outside buffer");
    data[offset] = static_cast<unsigned char>(value);
    data[offset + 1] = static_cast<unsigned char>(value >> 8);
}

void ValidateGameTableHeader(std::vector<unsigned char> const& data,
    GameTableSchema const& schema)
{
    // GameTables.cpp::LoadGameTable validates sizeof(T)/sizeof(float) values
    // plus the ignored first column. This is only the acquisition header gate;
    // numeric parsing and logical row coverage belong to the target consumer.
    size_t columns = 1;
    size_t end = 0;
    for (; end < data.size() && data[end] != '\n'; ++end)
    {
        if (data[end] == 0)
            throw std::runtime_error("GameTable header contains a NUL byte");
        if (data[end] == '\t') ++columns;
    }
    if (end == 0 || end == data.size() || columns != schema.valueColumns + 1)
        throw std::runtime_error("Unexpected initial GameTable header columns");
}

void SelfTestGameTableOracle()
{
    auto evidence = InitialGameTableOracle::Parse("id\tvalue\r\nignored\t1.5\t\r\n0\t-0\r\n\r\n1\t9\r\n", 1);
    InitialGameTableOracle::Evidence expected;
    InitialGameTableOracle::Feed(expected, 0.0f);
    InitialGameTableOracle::Feed(expected, 1.5f);
    InitialGameTableOracle::Feed(expected, -0.0f);
    if (evidence.rows != 3 || evidence.fingerprint != expected.fingerprint ||
        InitialGameTableOracle::Numeric("1oops") != 0.0f ||
        InitialGameTableOracle::Numeric("0x1p2") != 0.0f ||
        InitialGameTableOracle::Numeric("  +1.25") != 1.25f)
        throw std::runtime_error("Initial GameTable source oracle self-test failed");
}

void ValidateDb2HeaderSchema(std::vector<unsigned char> const& data,
    Db2Schema const& schema)
{
    ValidateHeader(data);
    DWORD headerFields = HeaderWord(data, 140);
    DWORD headerTotalFields = HeaderWord(data, 176);
    DWORD metadataFieldCount = headerTotalFields +
        (schema.parentIndexField >= static_cast<int>(headerTotalFields) ? 1u : 0u);
    if (HeaderWord(data, 156) != schema.layoutHash || headerFields != schema.fileFieldCount ||
        metadataFieldCount != schema.metaFieldCount)
        throw std::runtime_error(std::string("Unexpected ") + schema.fileName +
            " WDC5 layout/field metadata");
    if (schema.parentIndexField == -1 && HeaderWord(data, 184) != 0)
        throw std::runtime_error(std::string("Unexpected parent lookup in ") + schema.fileName);
}

void ValidateDb2Schema(std::vector<unsigned char> const& data,
    Db2Schema const& schema)
{
    ValidateDb2HeaderSchema(data, schema);
    if (HeaderHalfWord(data, 172) & Db2SparseFlag)
        throw std::runtime_error(std::string("Sparse ") + schema.fileName +
            " is not a complete normal DB2 table");

    DWORD sections = HeaderWord(data, 200);
    if (sections == 0)
    {
        // 02245dcd LoadHeaders:1788-1802, Load:1900-1915 permits
        // genuine zero-section tables with their complete field metadata.
        if (HeaderWord(data, 136) != 0 || HeaderWord(data, 148) != 0 ||
            HeaderWord(data, 188) != 0 || HeaderWord(data, 192) != 0 || HeaderWord(data, 196) != 0 ||
            data.size() != Db2HeaderSize + size_t(schema.fileFieldCount) * 4)
            throw std::runtime_error("Incomplete or contradictory empty DB2 table");
        return;
    }
    if (sections > 1024)
        throw std::runtime_error(std::string("Unsupported section count in ") + schema.fileName);
    ULONGLONG sectionBytes = ULONGLONG(sections) * Db2SectionHeaderSize;
    if (sectionBytes > ULONGLONG(data.size() - Db2HeaderSize))
        throw std::runtime_error(std::string("Truncated section headers in ") + schema.fileName);

    for (DWORD section = 0; section < sections; ++section)
    {
        size_t offset = Db2HeaderSize + size_t(section) * Db2SectionHeaderSize;
        DWORD records = HeaderWord(data, offset + 12);
        DWORD idBytes = HeaderWord(data, offset + 24);
        ULONGLONG expectedIdBytes = schema.indexField == -1 ? ULONGLONG(records) * 4 : 0;
        if (ULONGLONG(idBytes) != expectedIdBytes)
            throw std::runtime_error(std::string("Unexpected section ID table in ") + schema.fileName);
    }
}

void ValidateTactKey(std::vector<unsigned char> const& data)
{
    ValidateHeader(data);
    // TactKeyMeta in DB2Metadata.h (02245dcd) has one 16-byte field and an
    // external ID.  The target WDC5 header carries the corresponding
    // table/layout hashes and external-ID-list flag.  DB2Header::IndexField
    // is a separate native header field (0 in this file); the -1 external-ID
    // declaration belongs to DB2Meta, not to that header field.
    if (HeaderWord(data, 136) == 0 ||
        HeaderWord(data, 140) != TactKeyFieldCount ||
        HeaderWord(data, 144) != TactKeyRecordSize ||
        HeaderWord(data, 152) != TactKeyTableHash ||
        HeaderWord(data, 156) != TactKeyLayoutHash ||
        HeaderHalfWord(data, 172) & Db2SparseFlag ||
        !(HeaderHalfWord(data, 172) & Db2ExternalIdListFlag) ||
        HeaderWord(data, 176) != TactKeyFieldCount ||
        HeaderWord(data, 184) != 0 ||
        HeaderWord(data, 200) == 0)
        throw std::runtime_error("Unexpected TactKey WDC5 schema metadata");

    DWORD sectionCount = HeaderWord(data, 200);
    ULONGLONG sectionBytes = ULONGLONG(sectionCount) * Db2SectionHeaderSize;
    if (sectionBytes > ULONGLONG(data.size() - Db2HeaderSize))
        throw std::runtime_error("Truncated TactKey WDC5 section headers");
    for (DWORD section = 0; section < sectionCount; ++section)
    {
        ULONGLONG sectionOffset = ULONGLONG(Db2HeaderSize) +
            ULONGLONG(section) * Db2SectionHeaderSize;
        DWORD recordCount = HeaderWord(data, static_cast<size_t>(sectionOffset + 12));
        DWORD idListSize = HeaderWord(data, static_cast<size_t>(sectionOffset + 24));
        if (ULONGLONG(recordCount) * 4 != ULONGLONG(idListSize))
            throw std::runtime_error("Unexpected TactKey WDC5 external ID list size");
    }
}

void TestHeader()
{
    ItemPrefixes::SelfTest();
    BirthAbilityPrefix::SelfTest();
    SpellPrefixes::SelfTest();
    SelfTestGameTableOracle();
    std::vector<unsigned char> data(204, 0);
    data[0] = 'W'; data[1] = 'D'; data[2] = 'C'; data[3] = '5'; data[4] = 5;
    ValidateHeader(data);
    auto reject = [](std::vector<unsigned char> const& invalid)
    {
        try { ValidateHeader(invalid); }
        catch (std::runtime_error const&) { return; }
        throw std::runtime_error("Header self-test did not reject invalid input");
    };
    reject(std::vector<unsigned char>(203, 0));
    auto invalid = data; invalid[3] = '4'; reject(invalid);
    invalid = data; invalid[4] = 4; reject(invalid);
    invalid = data; invalid[184] = 2; reject(invalid);
    if (HeaderWord(std::vector<unsigned char>{0x78, 0x56, 0x34, 0x12}, 0) != 0x12345678)
        throw std::runtime_error("Header endian self-test failed");

    std::vector<unsigned char> tactKey(Db2HeaderSize + Db2SectionHeaderSize, 0);
    tactKey[0] = 'W'; tactKey[1] = 'D'; tactKey[2] = 'C'; tactKey[3] = '5';
    tactKey[4] = 5;
    auto putWord = [&tactKey](size_t offset, DWORD value)
    {
        tactKey[offset] = static_cast<unsigned char>(value);
        tactKey[offset + 1] = static_cast<unsigned char>(value >> 8);
        tactKey[offset + 2] = static_cast<unsigned char>(value >> 16);
        tactKey[offset + 3] = static_cast<unsigned char>(value >> 24);
    };
    auto putHalf = [&tactKey](size_t offset, unsigned short value)
    {
        tactKey[offset] = static_cast<unsigned char>(value);
        tactKey[offset + 1] = static_cast<unsigned char>(value >> 8);
    };
    putWord(136, 1);
    putWord(140, TactKeyFieldCount);
    putWord(144, TactKeyRecordSize);
    putWord(152, TactKeyTableHash);
    putWord(156, TactKeyLayoutHash);
    putHalf(172, Db2ExternalIdListFlag);
    putHalf(174, 0);
    putWord(176, TactKeyFieldCount);
    putWord(200, 1);
    putWord(Db2HeaderSize + 12, 1);
    putWord(Db2HeaderSize + 24, 4);
    ValidateTactKey(tactKey);
    auto rejectTactKey = [&tactKey](char const* message, auto mutate)
    {
        auto invalidTactKey = tactKey;
        mutate(invalidTactKey);
        try { ValidateTactKey(invalidTactKey); }
        catch (std::runtime_error const&) { return; }
        throw std::runtime_error(message);
    };
    rejectTactKey("TactKey self-test did not reject table hash", [](auto& value)
    {
        value[152] ^= 1;
    });
    rejectTactKey("TactKey self-test did not reject field count", [](auto& value)
    {
        value[140] = 2;
    });
    rejectTactKey("TactKey self-test did not reject external ID list flag", [](auto& value)
    {
        value[172] = 0;
    });
    rejectTactKey("TactKey self-test did not reject sparse flag", [](auto& value)
    {
        value[172] = static_cast<unsigned char>(Db2ExternalIdListFlag | Db2SparseFlag);
    });
    rejectTactKey("TactKey self-test did not reject record size", [](auto& value)
    {
        value[144] = 8;
    });
    rejectTactKey("TactKey self-test did not reject parent lookup", [](auto& value)
    {
        value[184] = 1;
    });
    rejectTactKey("TactKey self-test did not reject external ID list size", [](auto& value)
    {
        value[Db2HeaderSize + 24] = 0;
    });
    rejectTactKey("TactKey self-test did not reject truncated section", [](auto& value)
    {
        value.resize(Db2HeaderSize);
    });

    auto makeCharacterSchema = [](Db2Schema const& schema)
    {
        std::vector<unsigned char> value(Db2HeaderSize + Db2SectionHeaderSize, 0);
        value[0] = 'W'; value[1] = 'D'; value[2] = 'C'; value[3] = '5'; value[4] = 5;
        SetHeaderWord(value, 136, 1);
        SetHeaderWord(value, 140, schema.fileFieldCount);
        SetHeaderWord(value, 144, 16);
        SetHeaderWord(value, 156, schema.layoutHash);
        // External-ID storage is decided from DB2Meta::IndexField below, not
        // copied from TactKey's observed header flags.
        SetHeaderHalfWord(value, 172, 0);
        // The native DB2Header::IndexField is not DB2Meta::IndexField.
        SetHeaderHalfWord(value, 174, 0);
        SetHeaderWord(value, 176, schema.fileFieldCount +
            (schema.parentIndexField >= static_cast<int>(schema.fileFieldCount) ? 1u : 0u));
        SetHeaderWord(value, 200, 1);
        SetHeaderWord(value, Db2HeaderSize + 12, 1);
        SetHeaderWord(value, Db2HeaderSize + 24, schema.indexField == -1 ? 4 : 0);
        return value;
    };
    auto rejectCharacterSchema = [](char const* message, auto const& valid, auto mutate)
    {
        auto invalidSchema = valid;
        mutate(invalidSchema);
        try { ValidateDb2Schema(invalidSchema, CharacterCustomizationSchemas[0]); }
        catch (std::runtime_error const&) { return; }
        throw std::runtime_error(message);
    };
    auto expectCharacterReject = [](char const* message,
        std::vector<unsigned char> const& invalid, Db2Schema const& schema)
    {
        try { ValidateDb2Schema(invalid, schema); }
        catch (std::runtime_error const&) { return; }
        throw std::runtime_error(message);
    };
    auto inlineSchema = makeCharacterSchema(CharacterCustomizationSchemas[0]);
    ValidateDb2Schema(inlineSchema, CharacterCustomizationSchemas[0]);
    auto externalSchema = makeCharacterSchema(CharacterCustomizationSchemas[3]);
    ValidateDb2Schema(externalSchema, CharacterCustomizationSchemas[3]);
    auto parentFieldSchema = makeCharacterSchema(CharacterCustomizationSchemas[5]);
    ValidateDb2Schema(parentFieldSchema, CharacterCustomizationSchemas[5]);
    rejectCharacterSchema("Character schema self-test did not reject layout", inlineSchema,
        [](auto& value) { value[156] ^= 1; });
    rejectCharacterSchema("Character schema self-test did not reject file field count", inlineSchema,
        [](auto& value) { value[140] = 16; });
    rejectCharacterSchema("Character schema self-test did not reject total field count", inlineSchema,
        [](auto& value) { value[176] = 16; });
    rejectCharacterSchema("Character schema self-test did not reject sparse table", inlineSchema,
        [](auto& value) { value[172] = Db2SparseFlag; });
    rejectCharacterSchema("Character schema self-test did not reject inline ID table", inlineSchema,
        [](auto& value) { value[Db2HeaderSize + 24] = 4; });
    auto rejectExternalIdTable = externalSchema;
    rejectExternalIdTable[Db2HeaderSize + 24] = 0;
    expectCharacterReject("Character schema self-test did not reject external ID table", rejectExternalIdTable,
        CharacterCustomizationSchemas[3]);
    auto rejectParentLookup = makeCharacterSchema(CharacterCustomizationSchemas[3]);
    SetHeaderWord(rejectParentLookup, 184, 1);
    expectCharacterReject("Character schema self-test did not reject parent lookup", rejectParentLookup,
        CharacterCustomizationSchemas[3]);

    auto expectNameValidationReject = [](char const* message,
        std::vector<unsigned char> const& invalid, Db2Schema const& schema)
    {
        try { ValidateDb2Schema(invalid, schema); }
        catch (std::runtime_error const&) { return; }
        throw std::runtime_error(message);
    };
    auto checkSchema = [&](Db2Schema const& schema)
    {
        auto valid = makeCharacterSchema(schema);
        ValidateDb2Schema(valid, schema);

        auto invalid = valid;
        invalid[156] ^= 1;
        expectNameValidationReject("Name-validation self-test did not reject layout", invalid, schema);
        invalid = valid;
        invalid[140] = 0;
        expectNameValidationReject("Name-validation self-test did not reject field count", invalid, schema);
        invalid = valid;
        invalid[Db2HeaderSize + 24] = 0;
        expectNameValidationReject("Name-validation self-test did not reject external ID table", invalid, schema);
        invalid = valid;
        invalid[184] = 1;
        expectNameValidationReject("Name-validation self-test did not reject parent lookup", invalid, schema);
        invalid.resize(Db2HeaderSize);
        expectNameValidationReject("Name-validation self-test did not reject truncation", invalid, schema);
    };
    for (Db2Schema const& schema : NameValidationSchemas) checkSchema(schema);
    for (Db2Schema const& schema : CharacterInitializationSchemas)
    {
        auto valid = makeCharacterSchema(schema);
        ValidateDb2Schema(valid, schema);
        auto invalid = valid;
        invalid[156] ^= 1;
        expectNameValidationReject("Initialization self-test did not reject layout", invalid, schema);
        invalid = valid;
        SetHeaderWord(invalid, Db2HeaderSize + 24, schema.indexField == -1 ? 0 : 4);
        expectNameValidationReject("Initialization self-test did not reject ID source", invalid, schema);
        invalid = valid;
        SetHeaderWord(invalid, 140, schema.fileFieldCount - 1);
        expectNameValidationReject("Initialization self-test did not reject field count", invalid, schema);
        invalid = valid;
        invalid[172] = Db2SparseFlag;
        expectNameValidationReject("Initialization self-test did not reject sparse data", invalid, schema);
    }
    for (Db2Schema const& schema : CharacterBirthSchemas)
    {
        auto valid = makeCharacterSchema(schema);
        ValidateDb2Schema(valid, schema);
        auto invalid = valid;
        invalid[156] ^= 1;
        expectNameValidationReject("Birth self-test did not reject layout", invalid, schema);
        invalid = valid;
        SetHeaderWord(invalid, Db2HeaderSize + 24, schema.indexField == -1 ? 0 : 4);
        expectNameValidationReject("Birth self-test did not reject ID source", invalid, schema);
        invalid = valid;
        SetHeaderWord(invalid, 140, schema.fileFieldCount - 1);
        expectNameValidationReject("Birth self-test did not reject physical field count", invalid, schema);
        invalid = valid;
        SetHeaderWord(invalid, 176, schema.metaFieldCount - 1);
        expectNameValidationReject("Birth self-test did not reject metadata field count", invalid, schema);
        invalid = valid;
        invalid[172] = Db2SparseFlag;
        expectNameValidationReject("Birth self-test did not reject sparse data", invalid, schema);
        invalid.resize(Db2HeaderSize);
        expectNameValidationReject("Birth self-test did not reject truncated sections", invalid, schema);
        if (schema.parentIndexField == -1)
        {
            invalid = valid;
            SetHeaderWord(invalid, 184, 1);
            expectNameValidationReject("Birth self-test did not reject unexpected parent", invalid, schema);
        }
    }
    std::vector<Db2Schema> itemSchemas(CharacterItemSchemas.begin(), CharacterItemSchemas.end());
    itemSchemas.insert(itemSchemas.end(), ItemTemplateSchemas.begin(), ItemTemplateSchemas.end());
    for (Db2Schema const& schema : itemSchemas)
    {
        auto valid = makeCharacterSchema(schema);
        ValidateDb2Schema(valid, schema);
        auto invalid = valid;
        invalid[156] ^= 1;
        expectNameValidationReject("Item self-test did not reject layout", invalid, schema);
        invalid = valid;
        SetHeaderWord(invalid, Db2HeaderSize + 24, 0);
        expectNameValidationReject("Item self-test did not reject ID source", invalid, schema);
        invalid = valid;
        SetHeaderWord(invalid, 140, schema.fileFieldCount - 1);
        expectNameValidationReject("Item self-test did not reject field count", invalid, schema);
        invalid = valid;
        invalid[172] = Db2SparseFlag;
        // Sparse metadata is observable, but must never be certified as a
        // supported normal-record acquisition by the complete-file gate.
        ValidateDb2HeaderSchema(invalid, schema);
        expectNameValidationReject("Item self-test did not reject sparse data", invalid, schema);
        invalid.resize(Db2HeaderSize);
        expectNameValidationReject("Item self-test did not reject truncated sections", invalid, schema);
    }
    for (Db2Schema const& schema : ItemTemplateSchemas)
    {
        auto empty = makeCharacterSchema(schema);
        for (size_t at : {size_t(136), size_t(148), size_t(188), size_t(192), size_t(196), size_t(200)})
            SetHeaderWord(empty, at, 0);
        empty.resize(Db2HeaderSize + size_t(schema.fileFieldCount) * 4);
        ValidateDb2Schema(empty, schema);
        auto invalid = empty; SetHeaderWord(invalid, 136, 1);
        expectNameValidationReject("Empty item table admitted missing record", invalid, schema);
        invalid = empty; invalid.pop_back();
        expectNameValidationReject("Empty item table admitted missing field metadata", invalid, schema);
    }
    for (Db2Schema const& schema : SpellInfoSchemas)
    {
        auto valid = makeCharacterSchema(schema);
        ValidateDb2Schema(valid, schema);
        auto invalid = valid; invalid[156] ^= 1;
        expectNameValidationReject("Spell schema admitted wrong layout", invalid, schema);
        invalid = valid; SetHeaderWord(invalid, 140, schema.fileFieldCount - 1);
        expectNameValidationReject("Spell schema admitted wrong field count", invalid, schema);
        invalid = valid; SetHeaderWord(invalid, Db2HeaderSize + 24, schema.indexField == -1 ? 0 : 4);
        expectNameValidationReject("Spell schema admitted wrong ID source", invalid, schema);
        invalid = valid; invalid[172] = Db2SparseFlag;
        ValidateDb2HeaderSchema(invalid, schema); // Observation only, never full sparse support.
        expectNameValidationReject("Spell schema admitted sparse records", invalid, schema);
        invalid.resize(Db2HeaderSize);
        expectNameValidationReject("Spell schema admitted truncated sections", invalid, schema);
        // Genuine empty tables still need primitive field metadata.
        auto empty = makeCharacterSchema(schema);
        for (size_t at : {size_t(136), size_t(148), size_t(188), size_t(192), size_t(196), size_t(200)})
            SetHeaderWord(empty, at, 0);
        empty.resize(Db2HeaderSize + size_t(schema.fileFieldCount) * 4);
        ValidateDb2Schema(empty, schema);
        invalid = empty; invalid.pop_back();
        expectNameValidationReject("Empty spell table admitted missing field metadata", invalid, schema);
    }
    for (auto const& group : {InitialGameTables, SpellValueGameTables})
    for (GameTableSchema const& schema : group)
    {
        std::string text = "ignored";
        for (size_t i = 0; i < schema.valueColumns; ++i) text += "\tvalue";
        text += "\r\n1\t0\r\n";
        std::vector<unsigned char> valid(text.begin(), text.end());
        ValidateGameTableHeader(valid, schema);
        auto rejected = [&](auto const& invalid)
        {
            try { ValidateGameTableHeader(invalid, schema); }
            catch (std::runtime_error const&) { return; }
            throw std::runtime_error("GameTable self-test accepted invalid header");
        };
        rejected(std::vector<unsigned char>{});
        rejected(std::vector<unsigned char>{'a', '\n'});
        auto invalid = valid;
        invalid[0] = 0;
        rejected(invalid);
        rejected(std::vector<unsigned char>{'a'});
    }
    // Synthetic header/zero body only, never target Map records or TACT IDs.
    std::vector<unsigned char> map(MapAvailableBytes, 0);
    SetHeaderWord(map, 0, 0x35434457); SetHeaderWord(map, 4, 5);
    SetHeaderWord(map, 136, 79); SetHeaderWord(map, 140, 26);
    SetHeaderWord(map, 144, 48); SetHeaderWord(map, 152, 0xBD84CD62);
    SetHeaderWord(map, 156, 0xD43AFAC3); SetHeaderHalfWord(map, 172, 4);
    SetHeaderWord(map, 176, 26); SetHeaderWord(map, 200, 4);
    constexpr std::array<DWORD, 4> mapRecords = {71, 2, 5, 1};
    constexpr std::array<DWORD, 4> mapOffsets = {2292, 9574, 9720, 10103};
    constexpr std::array<DWORD, 4> mapStrings = {3590, 42, 123, 26};
    for (DWORD i = 0; i < 4; ++i)
    {
        size_t at = Db2HeaderSize + i * Db2SectionHeaderSize;
        SetHeaderWord(map, at, i == 0 ? 0 : 1);
        SetHeaderWord(map, at + 8, mapOffsets[i]);
        SetHeaderWord(map, at + 12, mapRecords[i]);
        SetHeaderWord(map, at + 16, mapStrings[i]);
        SetHeaderWord(map, at + 24, mapRecords[i] * 4);
    }
    ValidateAvailableMap(map);
    auto rejectMap = [](std::vector<unsigned char> const& invalid)
    {
        try { ValidateAvailableMap(invalid); }
        catch (std::runtime_error const&) { return; }
        throw std::runtime_error("Available Map self-test accepted invalid prefix");
    };
    auto invalidMap = map; invalidMap.pop_back(); rejectMap(invalidMap);
    invalidMap = map; invalidMap.push_back(0); rejectMap(invalidMap);
    for (size_t at : {size_t(152), size_t(156), size_t(136), size_t(244), size_t(252), size_t(256)})
    {
        invalidMap = map; invalidMap[at] ^= 1; rejectMap(invalidMap);
    }
    std::cout << "WDC5/GameTable header self-test passed (base, TactKey, customization, name, initialization, birth and item contracts).\n";
}

fs::path ValidatePrivateTactKeyFile(fs::path const& input)
{
    fs::file_status linkStatus = fs::symlink_status(input);
    if (fs::is_symlink(linkStatus) || !fs::is_regular_file(linkStatus))
        throw std::runtime_error("TACT key input must be a regular non-symlink file");

    fs::path path = fs::canonical(input);
    fs::path root = fs::canonical(RUSTYCORE_QA_ROOT);
    std::string rootString = root.generic_string();
    std::string pathString = path.generic_string();
    if (pathString.compare(0, rootString.size(), rootString) != 0 ||
        (pathString.size() > rootString.size() && pathString[rootString.size()] != '/'))
        throw std::runtime_error("TACT key input must remain inside the ignored Forever fixture");

    fs::file_status status = fs::status(path);
    if (!fs::is_regular_file(status))
        throw std::runtime_error("TACT key input must remain a regular file");
    constexpr fs::perms mode0600 = fs::perms::owner_read | fs::perms::owner_write;
    constexpr fs::perms modeBits = fs::perms::owner_all | fs::perms::group_all |
        fs::perms::others_all;
    if ((status.permissions() & modeBits) != mode0600)
        throw std::runtime_error("TACT key input must have mode 0600");

    uintmax_t size = fs::file_size(path);
    // CascImportKeysFromFile rejects files >= 0xA00000; keep that source bound.
    if (size == 0 || size >= MaxTactKeyFileBytes)
        throw std::runtime_error("TACT key input size is outside the private bound");
    return path;
}

DWORD AchievementSectionWord(std::vector<unsigned char> const& data, DWORD section, size_t fieldOffset)
{
    return HeaderWord(data, Db2HeaderSize + size_t(section) * Db2SectionHeaderSize + fieldOffset);
}

ULONGLONG AchievementSectionTactId(std::vector<unsigned char> const& data, DWORD section)
{
    return HeaderLong(data, Db2HeaderSize + size_t(section) * Db2SectionHeaderSize);
}

void ValidateAvailableAchievement(std::vector<unsigned char> const& data)
{
    ValidateHeader(data);
    if (HeaderWord(data, 136) != AchievementRecordCount ||
        HeaderWord(data, 140) != AchievementFieldCount ||
        HeaderWord(data, 152) != AchievementTableHash ||
        HeaderWord(data, 156) != AchievementLayoutHash ||
        HeaderWord(data, 176) != AchievementFieldCount ||
        HeaderHalfWord(data, 172) != 0 || HeaderHalfWord(data, 174) != 3 ||
        HeaderWord(data, 200) != AchievementSectionCount)
        throw std::runtime_error("Unexpected Achievement WDC5 schema metadata");

    if (AchievementSectionTactId(data, 0) != 0 || AchievementSectionTactId(data, 1) == 0 ||
        AchievementSectionWord(data, 0, 8) != AchievementFirstSectionOffset ||
        AchievementSectionWord(data, 0, 12) != AchievementAvailableRecords ||
        AchievementSectionWord(data, 0, 16) != AchievementFirstSectionStringSize ||
        AchievementSectionWord(data, 0, 24) != 0 || AchievementSectionWord(data, 0, 36) != 0 ||
        AchievementSectionWord(data, 1, 8) != AchievementSecondSectionOffset ||
        AchievementSectionWord(data, 1, 12) != AchievementUnavailableRecords ||
        AchievementSectionWord(data, 1, 16) != AchievementSecondSectionStringSize ||
        AchievementSectionWord(data, 1, 24) != 0 || AchievementSectionWord(data, 1, 36) != 0)
        throw std::runtime_error("Unexpected Achievement WDC5 section metadata");

    ULONGLONG firstSectionEnd = AchievementSectionWord(data, 0, 8);
    firstSectionEnd += ULONGLONG(HeaderWord(data, 144)) * AchievementSectionWord(data, 0, 12);
    firstSectionEnd += AchievementSectionWord(data, 0, 16);
    firstSectionEnd += AchievementSectionWord(data, 0, 24);
    firstSectionEnd += AchievementSectionWord(data, 0, 28);
    firstSectionEnd += ULONGLONG(AchievementSectionWord(data, 0, 36)) * 8;
    if (firstSectionEnd != AchievementSecondSectionOffset)
        throw std::runtime_error("Achievement available-section boundary is inconsistent");
}

void ExtractAvailableAchievement(HANDLE storage, fs::path const& output, char const* locale)
{
    File file;
    // No CASC_OVERCOME_ENCRYPTED: the prefix must be read from the original
    // plaintext section and the unavailable section must remain excluded.
    if (!CascOpenFile(storage, CASC_FILE_DATA_ID(AchievementFileDataId), CASC_LOCALE_ENUS,
            CASC_OPEN_BY_FILEID, &file.value)) CascFailure("Open local Achievement DB2");
    ULONGLONG size = 0;
    if (!CascGetFileSize64(file.value, &size)) CascFailure("Read Achievement DB2 size");
    if (size < AchievementSecondSectionOffset || size > MaxBytes)
        throw std::runtime_error("Achievement DB2 size outside bound");
    if (!CascSetFilePointer64(file.value, 0, nullptr, FILE_BEGIN))
        CascFailure("Seek local Achievement DB2");
    std::vector<unsigned char> prefix(AchievementSecondSectionOffset);
    DWORD read = 0;
    if (!CascReadFile(file.value, prefix.data(), static_cast<DWORD>(prefix.size()), &read) ||
        read != prefix.size()) CascFailure("Read available Achievement DB2 prefix");
    ValidateAvailableAchievement(prefix);
    if (CascFindEncryptionKey(storage, AchievementSectionTactId(prefix, 1)))
        throw std::runtime_error("Available-prefix mode requires the second section's key to remain unavailable");
    SaveNew(output / "Achievement.available.db2", prefix);
    std::cout << "{\"build\":" << Build << ",\"file_id\":" << AchievementFileDataId
        << ",\"locale\":\"" << locale << "\",\"available_records\":"
        << AchievementAvailableRecords << ",\"unavailable_records\":"
        << AchievementUnavailableRecords << ",\"prefix_bytes\":" << prefix.size()
        << ",\"table_hash\":" << AchievementTableHash << ",\"layout_hash\":"
        << AchievementLayoutHash << ",\"magic\":\"WDC5\",\"version\":5"
        << ",\"source_loader\":\"Skip\",\"local_only\":true"
        << ",\"missing_key_zero_fill\":false}\n";
}

void Extract(HANDLE storage, DWORD id, char const* name, fs::path const& output,
    char const* locale, bool validateTactKey = false, Db2Schema const* expectedSchema = nullptr,
    char const* schemaGroup = "character_customization", bool metadataOnly = false)
{
    File file;
    // No missing-key zero-fill flag or remote storage; optional key import is
    // completed before extraction.
    if (!CascOpenFile(storage, CASC_FILE_DATA_ID(id), CASC_LOCALE_ENUS,
            CASC_OPEN_BY_FILEID, &file.value)) CascFailure("Open local DB2");
    ULONGLONG size = 0;
    if (!CascGetFileSize64(file.value, &size)) CascFailure("Read DB2 size");
    // Metadata mode allocates only the header + <=1024 section headers;
    // the complete-file read/write bound remains unchanged.
    if (size < 4 || (!metadataOnly && size > MaxBytes)) throw std::runtime_error("DB2 size outside bound");
    std::vector<unsigned char> header(204);
    DWORD headerRead = 0;
    if (!CascReadFile(file.value, header.data(), static_cast<DWORD>(header.size()), &headerRead) || headerRead != header.size())
        CascFailure("Read local DB2 header");
    ValidateHeader(header);
    std::cout << "{\"file_id\":" << id << ",\"header_only\":true,\"records\":" << HeaderWord(header, 136)
        << ",\"fields\":" << HeaderWord(header, 140) << ",\"record_size\":" << HeaderWord(header, 144)
        << ",\"table_hash\":" << HeaderWord(header, 152) << ",\"layout_hash\":" << HeaderWord(header, 156)
        << ",\"locale_mask\":" << HeaderWord(header, 168)
        << ",\"flags\":" << HeaderHalfWord(header, 172) << ",\"id_index\":" << HeaderHalfWord(header, 174)
        << ",\"total_fields\":" << HeaderWord(header, 176)
        << ",\"parent_lookup_count\":" << HeaderWord(header, 184)
        << ",\"storage_info_size\":" << HeaderWord(header, 188)
        << ",\"sections\":" << HeaderWord(header, 200) << "}\n";
    DWORD sections = HeaderWord(header, 200);
    if (sections > 1024) throw std::runtime_error("DB2 section count outside bound");
    std::vector<unsigned char> sectionHeaders(size_t(sections) * 40);
    DWORD sectionRead = 0;
    if (sections && (!CascReadFile(file.value, sectionHeaders.data(), static_cast<DWORD>(sectionHeaders.size()), &sectionRead) || sectionRead != sectionHeaders.size()))
        CascFailure("Read local DB2 section headers");
    bool unavailableSection = false;
    for (DWORD index = 0; index < sections; ++index)
    {
        size_t offset = size_t(index) * 40;
        ULONGLONG tact = HeaderLong(sectionHeaders, offset);
        bool keyAvailable = !tact || CascFindEncryptionKey(storage, tact);
        std::cout << "{\"file_id\":" << id << ",\"section\":" << index
            << ",\"records\":" << HeaderWord(sectionHeaders, offset + 12)
            << ",\"offset\":" << HeaderWord(sectionHeaders, offset + 8)
            << ",\"string_bytes\":" << HeaderWord(sectionHeaders, offset + 16)
            << ",\"id_bytes\":" << HeaderWord(sectionHeaders, offset + 24)
            << ",\"copies\":" << HeaderWord(sectionHeaders, offset + 36)
            << ",\"key_available\":" << (keyAvailable ? "true" : "false") << "}\n";
        if (expectedSchema && !keyAvailable)
            unavailableSection = true;
    }
    if (metadataOnly)
    {
        if (!expectedSchema) throw std::runtime_error("Metadata-only acquisition requires a source schema");
        header.insert(header.end(), sectionHeaders.begin(), sectionHeaders.end());
        ValidateDb2HeaderSchema(header, *expectedSchema);
        std::cout << "{\"file_id\":" << id << ",\"metadata_only\":true,\"file_saved\":false"
            << ",\"bytes\":" << size << ",\"all_section_keys_available\":"
            << (unavailableSection ? "false" : "true")
            << ",\"normal_reader_supported\":"
            << ((HeaderHalfWord(header, 172) & Db2SparseFlag) ? "false" : "true") << "}\n";
        return;
    }
    if (unavailableSection)
        throw std::runtime_error(std::string("Required ") +
            schemaGroup + " table " + name + " has an unavailable encrypted section");
    if (!CascSetFilePointer64(file.value, 0, nullptr, FILE_BEGIN)) CascFailure("Seek local DB2");
    std::vector<unsigned char> data(static_cast<size_t>(size));
    DWORD read = 0;
    if (!CascReadFile(file.value, data.data(), static_cast<DWORD>(size), &read) || read != size)
        CascFailure("Read complete local DB2");
    ValidateHeader(data);
    if (validateTactKey) ValidateTactKey(data);
    if (expectedSchema) ValidateDb2Schema(data, *expectedSchema);
    SaveNew(output / name, data);
    std::cout << "{\"build\":" << Build << ",\"file_id\":" << id
        << ",\"locale\":\"" << locale << "\",\"bytes\":" << size << ",\"magic\":\"WDC5\""
        << ",\"version\":5,\"records\":" << HeaderWord(data, 136)
        << ",\"fields\":" << HeaderWord(data, 140)
        << ",\"table_hash\":" << HeaderWord(data, 152)
        << ",\"layout_hash\":" << HeaderWord(data, 156)
        << ",\"local_only\":true,\"missing_key_zero_fill\":false";
    if (expectedSchema)
        std::cout << ",\"" << schemaGroup << "_schema\":true";
    std::cout << "}\n";
}

void ExtractGameTable(HANDLE storage, GameTableSchema const& schema, fs::path const& output)
{
    File file;
    if (!CascOpenFile(storage, CASC_FILE_DATA_ID(schema.fileDataId), CASC_LOCALE_NONE,
            CASC_OPEN_BY_FILEID, &file.value)) CascFailure("Open local initial GameTable");
    ULONGLONG size = 0;
    if (!CascGetFileSize64(file.value, &size)) CascFailure("Read GameTable size");
    if (size == 0 || size > MaxBytes) throw std::runtime_error("GameTable size outside bound");
    std::vector<unsigned char> data(static_cast<size_t>(size));
    DWORD read = 0;
    if (!CascReadFile(file.value, data.data(), static_cast<DWORD>(size), &read) || read != size)
        CascFailure("Read complete local initial GameTable");
    ValidateGameTableHeader(data, schema);
    auto evidence = InitialGameTableOracle::Parse(std::string(data.begin(), data.end()), schema.valueColumns);
    SaveNew(output / schema.fileName, data);
    std::cout << "{\"build\":" << Build << ",\"file_id\":" << schema.fileDataId
        << ",\"bytes\":" << size << ",\"value_columns\":" << schema.valueColumns
        << ",\"initial_game_table_header\":true,\"local_only\":true"
        << ",\"source_linux_gt_rows_including_unused_zero\":" << evidence.rows
        << ",\"source_linux_gt_fnv64\":" << evidence.fingerprint
        << ",\"missing_key_zero_fill\":false}\n";
}

void ValidateAvailableMap(std::vector<unsigned char> const& data)
{
    ValidateDb2Schema(data, InitialMapSchema);
    if (HeaderWord(data, 136) != 79 || HeaderWord(data, 144) != 48 ||
        HeaderWord(data, 152) != 0xBD84CD62 || HeaderHalfWord(data, 172) != 4 ||
        HeaderWord(data, 200) != 4 || data.size() != MapAvailableBytes)
        throw std::runtime_error("Unexpected available Map prefix schema");
    constexpr std::array<DWORD, 4> records = {71, 2, 5, 1};
    constexpr std::array<DWORD, 4> offsets = {2292, 9574, 9720, 10103};
    constexpr std::array<DWORD, 4> strings = {3590, 42, 123, 26};
    for (DWORD i = 0; i < 4; ++i)
    {
        size_t at = Db2HeaderSize + i * Db2SectionHeaderSize;
        if ((HeaderLong(data, at) == 0) != (i == 0) ||
            HeaderWord(data, at + 8) != offsets[i] ||
            HeaderWord(data, at + 12) != records[i] ||
            HeaderWord(data, at + 16) != strings[i] ||
            HeaderWord(data, at + 24) != records[i] * 4 ||
            HeaderWord(data, at + 28) != 0 || HeaderWord(data, at + 36) != 0)
            throw std::runtime_error("Unexpected available Map section boundary");
    }
}

void ExtractAvailableMap(HANDLE storage, fs::path const& output)
{
    File file;
    if (!CascOpenFile(storage, CASC_FILE_DATA_ID(1349477), CASC_LOCALE_NONE,
            CASC_OPEN_BY_FILEID, &file.value)) CascFailure("Open local available Map");
    std::vector<unsigned char> data(MapAvailableBytes);
    DWORD read = 0;
    if (!CascReadFile(file.value, data.data(), static_cast<DWORD>(data.size()), &read) || read != data.size())
        CascFailure("Read plaintext Map prefix");
    ValidateAvailableMap(data);
    for (DWORD i = 1; i < 4; ++i)
        if (CascFindEncryptionKey(storage, HeaderLong(data, Db2HeaderSize + i * Db2SectionHeaderSize)))
            throw std::runtime_error("Available Map mode requires all excluded sections to remain unavailable");
    SaveNew(output / "Map.available.db2", data);
    std::cout << "{\"build\":70170,\"file_id\":1349477,\"available_records\":71"
        << ",\"unavailable_records\":8,\"prefix_bytes\":" << data.size()
        << ",\"table_hash\":3179597154,\"layout_hash\":3560635075"
        << ",\"source_loader\":\"Skip\",\"local_only\":true,\"missing_key_zero_fill\":false}\n";
}

void ExtractAvailableBirthAbility(HANDLE storage, fs::path const& output)
{
    File file;
    if (!CascOpenFile(storage, CASC_FILE_DATA_ID(BirthAbilityPrefix::FileDataId), CASC_LOCALE_ENUS,
            CASC_OPEN_BY_FILEID, &file.value)) CascFailure("Open local birth abilities");
    ULONGLONG size = 0;
    if (!CascGetFileSize64(file.value, &size)) CascFailure("Read birth-ability size");
    if (size != BirthAbilityPrefix::FullBytes)
        throw std::runtime_error("Unexpected full birth-ability file extent");
    std::vector<unsigned char> prefix(BirthAbilityPrefix::AvailableBytes);
    DWORD read = 0;
    if (!CascReadFile(file.value, prefix.data(), static_cast<DWORD>(prefix.size()), &read) || read != prefix.size())
        CascFailure("Read plaintext birth-ability prefix");
    ValidateDb2Schema(prefix, CharacterBirthSchemas[2]);
    BirthAbilityPrefix::Validate(prefix);
    for (DWORD i = 1; i < 6; ++i)
        if (CascFindEncryptionKey(storage, HeaderLong(prefix, Db2HeaderSize + i * Db2SectionHeaderSize)))
            throw std::runtime_error("Available birth abilities require excluded sections to remain unavailable");
    SaveNew(output / "SkillLineAbility.available.db2", prefix);
    std::cout << "{\"build\":70170,\"file_id\":1266278,\"available_records\":7833,\"unavailable_records\":5"
        << ",\"prefix_bytes\":" << prefix.size() << ",\"table_hash\":" << BirthAbilityPrefix::TableHash
        << ",\"layout_hash\":575635104,\"character_birth_schema\":true,\"source_loader\":\"Skip\""
        << ",\"local_only\":true,\"missing_key_zero_fill\":false}\n";
}

void ExtractAvailableItem(HANDLE storage, fs::path const& output, ItemPrefixes::Contract const& c)
{
    File file;
    if (!CascOpenFile(storage, CASC_FILE_DATA_ID(c.fileId), CASC_LOCALE_ENUS,
        CASC_OPEN_BY_FILEID, &file.value)) CascFailure("Open local available item table");
    ULONGLONG size = 0;
    if (!CascGetFileSize64(file.value, &size)) CascFailure("Read local item-table size");
    if (size != c.fullBytes) throw std::runtime_error("Wrong full bounded item-table extent");
    // Only this exact ItemSparse prefix exceeds the ordinary 4 MiB bound.
    // No full-file/sparse normal-reader relaxation or encrypted zero fill.
    std::vector<unsigned char> bytes(c.prefixBytes);
    DWORD read = 0;
    if (!CascReadFile(file.value, bytes.data(), static_cast<DWORD>(bytes.size()), &read) || read != bytes.size())
        CascFailure("Read local plaintext item-table prefix");
    ItemPrefixes::Validate(bytes, c);
    for (uint32_t i = 1; i < c.sections; ++i)
        if (CascFindEncryptionKey(storage, HeaderLong(bytes, 204 + size_t(i)*40)))
            throw std::runtime_error("Available items require all excluded sections to remain unavailable");
    SaveNew(output / c.output, bytes);
    uint32_t unknown = HeaderWord(bytes, 136) - c.counts[0];
    std::cout << "{\"file_id\":" << c.fileId << ",\"available_records\":" << c.counts[0]
        << ",\"unavailable_records\":" << unknown << ",\"available_copies\":" << c.copies[0]
        << ",\"prefix_bytes\":" << c.prefixBytes << ",\"table_hash\":" << c.hash
        << ",\"layout_hash\":" << c.layout << ",\"source_loader\":\"Skip\""
        << ",\"local_only\":true,\"missing_key_zero_fill\":false}\n";
}

void ExtractAvailableSpell(HANDLE storage, fs::path const& output, SpellPrefixes::Contract const& c)
{
    auto const& schema = SpellInfoSchemas.at(c.schemaIndex);
    File file;
    if (!CascOpenFile(storage, CASC_FILE_DATA_ID(schema.fileDataId), CASC_LOCALE_ENUS,
        CASC_OPEN_BY_FILEID, &file.value)) CascFailure("Open local available spell table");
    ULONGLONG size = 0;
    if (!CascGetFileSize64(file.value, &size)) CascFailure("Read local spell-table size");
    if (size != c.fullBytes) throw std::runtime_error("Wrong bounded full spell-table extent");
    std::vector<unsigned char> bytes(c.prefixBytes);
    DWORD read = 0;
    if (!CascReadFile(file.value, bytes.data(), static_cast<DWORD>(bytes.size()), &read) || read != bytes.size())
        CascFailure("Read local plaintext spell-table prefix");
    ValidateDb2Schema(bytes, schema);
    SpellPrefixes::Validate(bytes, c);
    for (uint32_t i = 1; i < c.sections; ++i)
        if (CascFindEncryptionKey(storage, HeaderLong(bytes, 204 + size_t(i)*40)))
            throw std::runtime_error("Available spells require all excluded sections to remain unavailable");
    SaveNew(output / (fs::path(schema.fileName).stem().string() + ".available.db2"), bytes);
    std::cout << "{\"file_id\":" << schema.fileDataId << ",\"available_records\":" << c.knownRecords
        << ",\"unavailable_records\":" << c.records - c.knownRecords
        << ",\"available_copies\":" << c.copies << ",\"prefix_bytes\":" << bytes.size()
        << ",\"table_hash\":" << c.tableHash << ",\"layout_hash\":" << schema.layoutHash
        << ",\"spell_info_schema\":true,\"source_loader\":\"Skip\""
        << ",\"local_only\":true,\"missing_key_zero_fill\":false}\n";
}
}

int main(int argc, char** argv)
{
    try
    {
        if (argc == 2 && std::string(argv[1]) == "--self-test")
        {
            TestHeader();
            return 0;
        }
        auto options = ProbeOptions::Parse(argc, argv, ValidatePrivateTactKeyFile);
        std::string locale = argv[4];
        DWORD localeMask = locale == "esES" ? CASC_LOCALE_ESES :
            locale == "enUS" ? CASC_LOCALE_ENUS : 0;
        if (!localeMask) throw std::runtime_error("Unsupported acquisition locale");
        if (options.availableSpellInfoTables && locale != "esES")
            throw std::runtime_error("Available spell contracts require observed esES locale");
        fs::path storagePath = fs::canonical(argv[2]);
        fs::path output = fs::weakly_canonical(argv[3]);
        std::string privateRoot = fs::canonical(RUSTYCORE_QA_ROOT).generic_string() + "/";
        if (output.generic_string().compare(0, privateRoot.size(), privateRoot) != 0)
            throw std::runtime_error("Output must remain in the ignored Forever fixture");
        if (fs::exists(output)) throw std::runtime_error("Refusing to reuse an output directory");

        CASC_OPEN_STORAGE_ARGS args{};
        args.Size = sizeof(args);
        std::string path = storagePath.string();
        args.szLocalPath = path.c_str();
        args.szCodeName = "wow_classic_beta";
        // CascOpenFile ignores its locale argument in this pinned CascLib.
        // The root's locale must be selected when opening the storage instead.
        args.dwLocaleMask = localeMask;
        args.PfnProgressCallback = LocalOnly;
        Storage storage;
        if (!CascOpenStorageEx(nullptr, &args, false, &storage.value)) CascFailure("Open local storage");
        CASC_STORAGE_PRODUCT product{};
        if (!CascGetStorageInfo(storage.value, CascStorageProduct, &product, sizeof(product), nullptr))
            CascFailure("Read local product metadata");
        if (product.BuildNumber != Build || std::string(product.szCodeName) != "wow_classic_beta")
            throw std::runtime_error("Selected storage is not build-70170 Classic Beta");

        if (!options.tactKeyFile.empty())
        {
            // Matches extractor_common/CascHandles.cpp::LoadOnlineTactKeys, but
            // remains operator-supplied and offline; no key material is logged.
            if (!CascImportKeysFromFile(storage.value, options.tactKeyFile.c_str()))
                CascFailure("Import private TACT key list");
            std::cout << "{\"tact_keys_imported\":true,\"local_only\":true}\n";
        }

        fs::create_directories(output);
        fs::permissions(output, fs::perms::owner_all, fs::perm_options::replace);
        Extract(storage.value, 1361031, "ChrClasses.db2", output, locale.c_str());
        Extract(storage.value, 1305311, "ChrRaces.db2", output, locale.c_str());
        if (options.characterCustomizationTables)
            for (Db2Schema const& schema : CharacterCustomizationSchemas)
                Extract(storage.value, schema.fileDataId, schema.fileName, output, locale.c_str(), false, &schema);
        if (options.nameValidationTables)
            for (Db2Schema const& schema : NameValidationSchemas)
                Extract(storage.value, schema.fileDataId, schema.fileName, output, locale.c_str(), false,
                    &schema, "name_validation");
        if (options.characterInitializationTables)
        {
            fs::path gt = output / "gt";
            fs::create_directory(gt);
            fs::permissions(gt, fs::perms::owner_all, fs::perm_options::replace);
            for (GameTableSchema const& schema : InitialGameTables)
                ExtractGameTable(storage.value, schema, gt);
            for (Db2Schema const& schema : CharacterInitializationSchemas)
                if (schema.fileDataId == 1349477 && options.availableInitialMap)
                    ExtractAvailableMap(storage.value, output);
                else
                    Extract(storage.value, schema.fileDataId, schema.fileName, output, locale.c_str(), false,
                        &schema, "character_initialization");
        }
        if (options.characterBirthTables)
            for (Db2Schema const& schema : CharacterBirthSchemas)
                if (schema.fileDataId == BirthAbilityPrefix::FileDataId && options.availableBirthAbilities)
                    ExtractAvailableBirthAbility(storage.value, output);
                else
                    Extract(storage.value, schema.fileDataId, schema.fileName, output, locale.c_str(), false,
                        &schema, "character_birth");
        if (options.spellValueGameTables)
        {
            fs::path gt = output / "gt";
            if (!fs::exists(gt)) fs::create_directory(gt);
            fs::permissions(gt, fs::perms::owner_all, fs::perm_options::replace);
            for (GameTableSchema const& schema : SpellValueGameTables)
                ExtractGameTable(storage.value, schema, gt);
        }
        if (options.characterItemTables)
            if (options.availableItemTables)
                for (auto const& contract : ItemPrefixes::Contracts)
                    ExtractAvailableItem(storage.value, output, contract);
            else
                for (Db2Schema const& schema : CharacterItemSchemas)
                    Extract(storage.value, schema.fileDataId, schema.fileName, output, locale.c_str(), false,
                        &schema, "character_item", options.itemTableMetadataOnly);
        if (options.itemTemplateTables)
            for (Db2Schema const& schema : ItemTemplateSchemas)
                Extract(storage.value, schema.fileDataId, schema.fileName, output, locale.c_str(), false,
                    &schema, "item_template", options.itemTableMetadataOnly);
        if (options.spellInfoTables)
            for (size_t i = 0; i < SpellInfoSchemas.size(); ++i)
            {
                auto const& schema = SpellInfoSchemas[i];
                auto const* prefix = options.availableSpellInfoTables ? SpellPrefixes::Find(i) : nullptr;
                if (prefix) ExtractAvailableSpell(storage.value, output, *prefix);
                else Extract(storage.value, schema.fileDataId, schema.fileName, output, locale.c_str(), false,
                    &schema, "spell_info", options.spellTableMetadataOnly);
            }
        if (options.tactKeyTable)
            Extract(storage.value, TactKeyFileDataId, "TactKey.db2", output, locale.c_str(), true);
        if (options.availableAchievements)
            ExtractAvailableAchievement(storage.value, output, locale.c_str());
        else
            Extract(storage.value, AchievementFileDataId, "Achievement.db2", output, locale.c_str());
        return 0;
    }
    catch (std::exception const& error)
    {
        std::cerr << "Client data probe rejected: " << error.what() << '\n';
        return 1;
    }
}
