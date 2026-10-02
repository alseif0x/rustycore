// RustyCore operator-only acquisition of two local build-70170 DB2 tables.
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
constexpr ULONGLONG MaxBytes = 4 * 1024 * 1024;
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

void Extract(HANDLE storage, DWORD id, char const* name, fs::path const& output,
    char const* locale)
{
    File file;
    // No missing-key zero-fill flag, remote storage or imported key material.
    if (!CascOpenFile(storage, CASC_FILE_DATA_ID(id), CASC_LOCALE_ENUS,
            CASC_OPEN_BY_FILEID, &file.value)) CascFailure("Open local DB2");
    ULONGLONG size = 0;
    if (!CascGetFileSize64(file.value, &size)) CascFailure("Read DB2 size");
    if (size < 4 || size > MaxBytes) throw std::runtime_error("DB2 size outside bound");
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
        if (argc != 5 || std::string(argv[1]) != "--ack-local-client-data")
            throw std::runtime_error("Usage: --ack-local-client-data <WoW storage root> <new private output directory> <esES|enUS>");
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

        fs::create_directories(output);
        fs::permissions(output, fs::perms::owner_all, fs::perm_options::replace);
        Extract(storage.value, 1361031, "ChrClasses.db2", output, locale.c_str());
        Extract(storage.value, 1305311, "ChrRaces.db2", output, locale.c_str());
        return 0;
    }
    catch (std::exception const& error)
    {
        std::cerr << "Client data probe rejected: " << error.what() << '\n';
        return 1;
    }
}
