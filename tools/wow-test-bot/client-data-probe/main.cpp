// RustyCore operator-only acquisition of three local build-70170 DB2 tables.
// CascLib comes from the hash-pinned reference. No account or network access.
#include <CascLib.h>
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
    if (data.size() < 204 || HeaderWord(data, 0) != 0x35434457 || HeaderWord(data, 4) != 5)
        throw std::runtime_error("Expected complete WDC5/version-5 header");
    if (HeaderWord(data, 184) > 1)
        throw std::runtime_error("Unsupported DB2 parent lookup count");
}

void TestHeader()
{
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
    std::cout << "WDC5 header self-test passed (positive, truncation, magic, version, parent, endian).\n";
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
    char const* locale)
{
    File file;
    // No missing-key zero-fill flag or remote storage; optional key import is
    // completed before extraction.
    if (!CascOpenFile(storage, CASC_FILE_DATA_ID(id), CASC_LOCALE_ENUS,
            CASC_OPEN_BY_FILEID, &file.value)) CascFailure("Open local DB2");
    ULONGLONG size = 0;
    if (!CascGetFileSize64(file.value, &size)) CascFailure("Read DB2 size");
    if (size < 4 || size > MaxBytes) throw std::runtime_error("DB2 size outside bound");
    std::vector<unsigned char> header(204);
    DWORD headerRead = 0;
    if (!CascReadFile(file.value, header.data(), static_cast<DWORD>(header.size()), &headerRead) || headerRead != header.size())
        CascFailure("Read local DB2 header");
    ValidateHeader(header);
    std::cout << "{\"file_id\":" << id << ",\"header_only\":true,\"records\":" << HeaderWord(header, 136)
        << ",\"fields\":" << HeaderWord(header, 140) << ",\"table_hash\":" << HeaderWord(header, 152)
        << ",\"layout_hash\":" << HeaderWord(header, 156) << ",\"sections\":" << HeaderWord(header, 200) << "}\n";
    DWORD sections = HeaderWord(header, 200);
    if (sections > 1024) throw std::runtime_error("DB2 section count outside bound");
    std::vector<unsigned char> sectionHeaders(size_t(sections) * 40);
    DWORD sectionRead = 0;
    if (!CascReadFile(file.value, sectionHeaders.data(), static_cast<DWORD>(sectionHeaders.size()), &sectionRead) || sectionRead != sectionHeaders.size())
        CascFailure("Read local DB2 section headers");
    for (DWORD index = 0; index < sections; ++index)
    {
        size_t offset = size_t(index) * 40;
        ULONGLONG tact = HeaderLong(sectionHeaders, offset);
        std::cout << "{\"file_id\":" << id << ",\"section\":" << index
            << ",\"records\":" << HeaderWord(sectionHeaders, offset + 12)
            << ",\"offset\":" << HeaderWord(sectionHeaders, offset + 8)
            << ",\"string_bytes\":" << HeaderWord(sectionHeaders, offset + 16)
            << ",\"id_bytes\":" << HeaderWord(sectionHeaders, offset + 24)
            << ",\"copies\":" << HeaderWord(sectionHeaders, offset + 36)
            << ",\"key_available\":" << (!tact || CascFindEncryptionKey(storage, tact) ? "true" : "false") << "}\n";
    }
    if (!CascSetFilePointer64(file.value, 0, nullptr, FILE_BEGIN)) CascFailure("Seek local DB2");
    std::vector<unsigned char> data(static_cast<size_t>(size));
    DWORD read = 0;
    if (!CascReadFile(file.value, data.data(), static_cast<DWORD>(size), &read) || read != size)
        CascFailure("Read complete local DB2");
    ValidateHeader(data);
    SaveNew(output / name, data);
    std::cout << "{\"build\":" << Build << ",\"file_id\":" << id
        << ",\"locale\":\"" << locale << "\",\"bytes\":" << size << ",\"magic\":\"WDC5\""
        << ",\"version\":5,\"records\":" << HeaderWord(data, 136)
        << ",\"fields\":" << HeaderWord(data, 140)
        << ",\"table_hash\":" << HeaderWord(data, 152)
        << ",\"layout_hash\":" << HeaderWord(data, 156)
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
        if (argc < 5 || argc > 8 || std::string(argv[1]) != "--ack-local-client-data")
            throw std::runtime_error("Usage: --ack-local-client-data <WoW storage root> <new private output directory> <esES|enUS> [--ack-public-tact-keys <private-file>] [--ack-available-achievements]");
        fs::path tactKeyFile;
        bool tactKeyOptionSeen = false;
        bool availableAchievements = false;
        for (int argument = 5; argument < argc;)
        {
            std::string option = argv[argument];
            if (option == "--ack-public-tact-keys")
            {
                if (tactKeyOptionSeen || argument + 1 >= argc)
                    throw std::runtime_error("TACT key option requires one private file and may appear once");
                tactKeyOptionSeen = true;
                tactKeyFile = ValidatePrivateTactKeyFile(argv[argument + 1]);
                argument += 2;
            }
            else if (option == "--ack-available-achievements")
            {
                if (availableAchievements)
                    throw std::runtime_error("Available Achievement option may appear once");
                availableAchievements = true;
                ++argument;
            }
            else
                throw std::runtime_error("Unknown optional client-data probe argument");
        }
        std::string locale = argv[4];
        DWORD localeMask = locale == "esES" ? CASC_LOCALE_ESES :
            locale == "enUS" ? CASC_LOCALE_ENUS : 0;
        if (!localeMask) throw std::runtime_error("Unsupported acquisition locale");
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

        if (!tactKeyFile.empty())
        {
            // Matches extractor_common/CascHandles.cpp::LoadOnlineTactKeys, but
            // remains operator-supplied and offline; no key material is logged.
            if (!CascImportKeysFromFile(storage.value, tactKeyFile.c_str()))
                CascFailure("Import private TACT key list");
            std::cout << "{\"tact_keys_imported\":true,\"local_only\":true}\n";
        }

        fs::create_directories(output);
        fs::permissions(output, fs::perms::owner_all, fs::perm_options::replace);
        Extract(storage.value, 1361031, "ChrClasses.db2", output, locale.c_str());
        Extract(storage.value, 1305311, "ChrRaces.db2", output, locale.c_str());
        if (availableAchievements)
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
