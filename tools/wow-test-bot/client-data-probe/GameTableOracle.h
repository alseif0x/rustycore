// Read-only Linux numeric oracle for the three initialization text tables.
// 02245dcd GameTables.cpp:44-109, Util.cpp:57-74,797-804 and
// StringConvert.h:233-258. This does NOT establish Player formula parity.
#pragma once
#include <cstdint>
#include <cstring>
#include <sstream>
#include <stdexcept>
#include <string>
#include <string_view>
#include <vector>

namespace InitialGameTableOracle
{
struct Evidence
{
    std::size_t rows = 1; // explicit unused row zero
    std::uint64_t fingerprint = 14695981039346656037ULL;
};

inline std::vector<std::string_view> Tokens(std::string_view line, bool keepEmpty)
{
    std::vector<std::string_view> values;
    std::size_t start = 0;
    for (std::size_t end = line.find('\t'); end != std::string_view::npos;
        end = line.find('\t', start))
    {
        if (keepEmpty || start < end) values.push_back(line.substr(start, end - start));
        start = end + 1;
    }
    if (keepEmpty || start < line.size()) values.push_back(line.substr(start));
    return values;
}

inline float Numeric(std::string_view cell)
{
    // Source uses std::stold (long double), a fully-consumed result, then
    // casts to float; conversion failure falls back to +0. Unlike the Rust
    // asset admission parser, this oracle does not impose finite-decimal rules.
    if (cell.empty() || cell.substr(0, 2) == "0x" || cell.substr(0, 2) == "0X") return 0.0f;
    try
    {
        std::string text(cell);
        std::size_t consumed;
        float value = static_cast<float>(std::stold(text, &consumed));
        return consumed == text.size() ? value : 0.0f;
    }
    catch (...) { return 0.0f; }
}

inline void Feed(Evidence& evidence, float value)
{
    static_assert(sizeof(float) == sizeof(std::uint32_t));
    std::uint32_t bits;
    std::memcpy(&bits, &value, sizeof(bits));
    // Canonical LE bits, independent of host byte order; not a secret digest
    // or a cryptographic integrity proof. Never emit numeric cells.
    for (unsigned shift = 0; shift < 32; shift += 8)
    {
        evidence.fingerprint ^= (bits >> shift) & 255u;
        evidence.fingerprint *= 1099511628211ULL;
    }
}

inline Evidence Parse(std::string const& text, std::size_t columns)
{
    std::istringstream stream(text);
    std::string line;
    if (!std::getline(stream, line) || Tokens(line, false).size() != columns + 1)
        throw std::runtime_error("Initial GameTable oracle header mismatch");
    Evidence evidence;
    for (std::size_t i = 0; i < columns; ++i) Feed(evidence, 0.0f);
    while (std::getline(stream, line))
    {
        if (auto at = line.find_first_of("\r\n"); at != std::string::npos) line.erase(at);
        auto values = Tokens(line, true);
        while (values.size() > 1 && values.back().empty()) values.pop_back();
        if (values.size() <= 1) break;
        if (values.size() != columns + 1)
            throw std::runtime_error("Initial GameTable oracle row mismatch");
        for (std::size_t i = 1; i < values.size(); ++i) Feed(evidence, Numeric(values[i]));
        ++evidence.rows;
    }
    return evidence;
}
}
