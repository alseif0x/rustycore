// Temporary replay of Player::_LoadSkills' separate local map only.
#include "abi.hpp"

#include <algorithm>
#include <cstdint>
#include <unordered_map>
#include <vector>

extern "C" int rustycore_forever_loaded_skill_values_order(
    std::uint32_t const* first_assignments, std::size_t length,
    std::uint32_t* output, std::size_t capacity, std::size_t* written) noexcept
{
    if (written)
        *written = 0;
    if (!written || (length != 0 && !first_assignments) || (capacity != 0 && !output))
        return 1;

    try
    {
        // Keep the source's default container and insertion history. In
        // particular, do not reserve: rehash behavior is part of this replay.
        std::unordered_map<std::uint32_t, std::uint32_t> loadedSkillValues;
        for (std::size_t index = 0; index < length; ++index)
        {
            std::uint32_t const skill = first_assignments[index];
            if (loadedSkillValues.find(skill) != loadedSkillValues.end())
                return 1;
            loadedSkillValues[skill] = 0;
        }
        if (capacity != loadedSkillValues.size())
            return 1;

        std::vector<std::uint32_t> staged;
        staged.reserve(loadedSkillValues.size());
        for (auto const& [skill, value] : loadedSkillValues)
        {
            (void)value;
            staged.push_back(skill);
        }
        if (!staged.empty())
            std::copy(staged.begin(), staged.end(), output);
        *written = staged.size();
        return 0;
    }
    catch (...)
    {
        return 2;
    }
}
