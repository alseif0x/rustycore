// 02245dcd Player.h:687-704 SkillStatusMap; successful insertions only.
// Numeric temporary replay, never a native Player/status payload owner.
// The existing bridge translation unit enforces the pinned GNU toolchain.
#include "abi.hpp"
#include <unordered_map>

extern "C" int rustycore_forever_skill_status_order(
    std::uint32_t const* insertions, std::size_t length,
    std::uint32_t* output, std::size_t capacity, std::size_t* written) noexcept
{
    if (!written)
        return 1;
    *written = 0;
    if ((length && !insertions) || (capacity && !output))
        return 1;
    try
    {
        std::unordered_map<std::uint32_t, std::uint8_t> entries;
        for (std::size_t i = 0; i < length; ++i)
            if (!entries.try_emplace(insertions[i]).second)
                return 1;
        if (entries.size() != capacity)
            return 1;
        // All allocation/validation finished. These writes cannot throw; an
        // earlier invalid input or exception leaves output entirely untouched.
        std::size_t at = 0;
        for (auto const& [id, unused] : entries)
            output[at++] = id;
        *written = at;
        return 0;
    }
    catch (...)
    {
        return 2;
    }
}
