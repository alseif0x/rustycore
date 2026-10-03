// Replays source-style Linux numeric parsing on existing private GT assets.
// No CASC, account, SQL, network or output-file writes.
#include "GameTableOracle.h"
#include <array>
#include <cerrno>
#include <filesystem>
#include <iostream>
#include <fcntl.h>
#include <sys/stat.h>
#include <unistd.h>

namespace fs = std::filesystem;
namespace
{
struct Descriptor
{
    int value;
    ~Descriptor() { if (value >= 0) close(value); }
};

std::string Read(fs::path const& path)
{
    Descriptor fd{open(path.c_str(), O_RDONLY | O_NOFOLLOW)};
    struct stat metadata{};
    if (fd.value < 0 || fstat(fd.value, &metadata) != 0 ||
        !S_ISREG(metadata.st_mode) || (metadata.st_mode & 0777) != 0600 ||
        metadata.st_size <= 0 || metadata.st_size > 4 * 1024 * 1024)
        throw std::runtime_error("Private GT input rejected");
    std::string text(static_cast<std::size_t>(metadata.st_size), '\0');
    std::size_t done = 0;
    while (done < text.size())
    {
        auto count = read(fd.value, text.data() + done, text.size() - done);
        if (count < 0 && errno == EINTR) continue;
        if (count <= 0) throw std::runtime_error("Private GT read failed");
        done += static_cast<std::size_t>(count);
    }
    return text;
}
}

int main(int argc, char** argv)
{
    try
    {
        bool spellValues = argc == 3 && std::string_view(argv[1]) == "--ack-private-spell-value-gt-oracle";
        if (argc != 3 || (!spellValues && std::string_view(argv[1]) != "--ack-private-initial-gt-oracle"))
            throw std::runtime_error("Explicit private GT oracle acknowledgement required");
        fs::path input(argv[2]);
        if (fs::is_symlink(fs::symlink_status(input)))
            throw std::runtime_error("Private GT directory must not be a symlink");
        fs::path directory = fs::canonical(input);
        std::string root = fs::canonical(RUSTYCORE_QA_ROOT).generic_string() + "/";
        if (directory.generic_string().compare(0, root.size(), root) != 0 ||
            !fs::is_directory(directory) || fs::is_symlink(fs::symlink_status(directory / "gt")))
            throw std::runtime_error("Private GT directory must remain inside fixture root");
        auto names = spellValues ? std::array<char const*, 3>{"SpellScaling.txt", "CombatRatingsMultByILvl.txt", "StaminaMultByILvl.txt"}
            : std::array<char const*, 3>{"BaseMp.txt", "HpPerSta.txt", "xp.txt"};
        auto columns = spellValues ? std::array<std::size_t, 3>{24, 4, 4}
            : std::array<std::size_t, 3>{15, 1, 5};
        std::array<InitialGameTableOracle::Evidence, 3> evidence;
        for (std::size_t i = 0; i < names.size(); ++i)
            evidence[i] = InitialGameTableOracle::Parse(Read(directory / "gt" / names[i]), columns[i]);
        for (std::size_t i = 0; i < names.size(); ++i)
            std::cout << "{\"table\":\"" << names[i] << "\",\"source_linux_gt_rows_including_unused_zero\":"
                << evidence[i].rows << ",\"source_linux_gt_fnv64\":" << evidence[i].fingerprint << "}\n";
        return 0;
    }
    catch (...) // never emit filesystem paths or private contents
    {
        std::cerr << "Private initial GT oracle rejected\n";
        return 1;
    }
}
