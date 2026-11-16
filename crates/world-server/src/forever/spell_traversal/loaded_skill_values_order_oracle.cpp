// Independent replay against the actual extracted _LoadSkills local map declaration.
#include "abi.hpp"
#include "SourceLoadedSkillValues.hpp"

#include <algorithm>
#include <cstdint>
#include <limits>
#include <map>
#include <set>
#include <tuple>
#include <vector>

namespace
{
using LoadedSkillMap = SourceLoadedSkillValues;

std::vector<std::uint32_t> source_order(LoadedSkillMap const& source)
{
    std::vector<std::uint32_t> output;
    for (auto const& [skill, value] : source.loadedSkillValues)
    {
        (void)value;
        output.push_back(skill);
    }
    return output;
}

void assign(LoadedSkillMap& source, std::set<std::uint32_t>& first_seen,
    std::vector<std::uint32_t>& first_assignments,
    std::map<std::uint32_t, std::uint32_t>& expected_values,
    std::uint32_t skill, std::uint32_t value)
{
    if (first_seen.insert(skill).second)
        first_assignments.push_back(skill);
    source.loadedSkillValues[skill] = value;
    expected_values[skill] = value;
}

bool values_match(LoadedSkillMap const& source,
    std::map<std::uint32_t, std::uint32_t> const& expected_values)
{
    if (source.loadedSkillValues.size() != expected_values.size())
        return false;
    for (auto const& [skill, value] : expected_values)
    {
        auto const found = source.loadedSkillValues.find(skill);
        if (found == source.loadedSkillValues.end() || found->second != value)
            return false;
    }
    return true;
}

bool matches_abi(LoadedSkillMap const& source,
    std::vector<std::uint32_t> const& first_assignments)
{
    std::vector<std::uint32_t> const expected = source_order(source);
    std::vector<std::uint32_t> actual(expected.size(), 0xfeedfaceu);
    std::size_t written = 77;
    int const code = rustycore_forever_loaded_skill_values_order(
        first_assignments.data(), first_assignments.size(), actual.data(), actual.size(), &written);
    return code == 0 && written == expected.size() && actual == expected;
}

std::uint32_t key_for(std::size_t index, unsigned mode)
{
    auto const value = static_cast<std::uint32_t>(index);
    switch (mode)
    {
        case 0: return value;
        case 1: return value * 131u;
        case 2: return std::numeric_limits<std::uint32_t>::max() - value;
        case 3: return 0x80000000u ^ (value * 257u);
        default: return (value << 16) | (value ^ 0xa5a5u);
    }
}

bool compare(std::size_t length, unsigned mode)
{
    LoadedSkillMap source;
    std::set<std::uint32_t> first_seen;
    std::vector<std::uint32_t> first_assignments;
    std::map<std::uint32_t, std::uint32_t> expected_values;
    std::vector<std::uint32_t> inserted_keys;
    inserted_keys.reserve(length);

    for (std::size_t index = 0; index < length; ++index)
    {
        std::uint32_t const skill = key_for(index, mode);
        inserted_keys.push_back(skill);
        std::uint32_t const value = static_cast<std::uint32_t>(index * 37u + 11u);
        assign(source, first_seen, first_assignments, expected_values, skill, value);

        // Interleave updates to an older node with new insertions, including
        // histories that cross multiple default-container rehash thresholds.
        if (index >= 2 && index % 3 == 0)
        {
            std::uint32_t const earlier = inserted_keys[index / 2];
            assign(source, first_seen, first_assignments, expected_values,
                earlier, 0x90000000u | static_cast<std::uint32_t>(index));
        }
        if (index % 7 == 0)
            assign(source, first_seen, first_assignments, expected_values,
                skill, value ^ 0x5a5a5a5au);
    }

    std::vector<std::uint32_t> const before_late_overwrite = source_order(source);
    if (!first_assignments.empty())
    {
        std::uint32_t const existing = first_assignments[first_assignments.size() / 2];
        assign(source, first_seen, first_assignments, expected_values, existing, 0xfedcba98u);
        if (source_order(source) != before_late_overwrite)
            return false;
    }

    return first_assignments.size() == source.loadedSkillValues.size()
        && values_match(source, expected_values)
        && matches_abi(source, first_assignments);
}

bool independent_lifecycles()
{
    LoadedSkillMap first;
    LoadedSkillMap second;
    std::set<std::uint32_t> first_seen, second_seen;
    std::vector<std::uint32_t> first_history, second_history;
    std::map<std::uint32_t, std::uint32_t> first_values, second_values;
    for (auto const skill : {0u, 1u, 65536u, std::numeric_limits<std::uint32_t>::max()})
        assign(first, first_seen, first_history, first_values, skill, skill + 17u);
    for (auto const skill : {7u, 0x80000000u, 0xfffffff0u})
        assign(second, second_seen, second_history, second_values, skill, skill ^ 0x1234u);
    return values_match(first, first_values) && values_match(second, second_values)
        && matches_abi(first, first_history) && matches_abi(second, second_history)
        && matches_abi(first, first_history);
}

bool invalid_contract(std::size_t& cases)
{
    std::uint32_t const duplicate[] = {7u, 7u};
    std::uint32_t const unique[] = {7u, 8u};
    std::uint32_t output[] = {99u, 99u, 99u};
    std::size_t written = 77;
    for (auto const [history, length, capacity] : {
        std::tuple<std::uint32_t const*, std::size_t, std::size_t>{duplicate, 2, 2},
        {duplicate, 2, 1}, {unique, 2, 1}, {unique, 2, 3}})
    {
        std::fill(output, output + 3, 99u);
        written = 77;
        if (rustycore_forever_loaded_skill_values_order(
                history, length, output, capacity, &written) != 1
            || written != 0 || output[0] != 99u || output[1] != 99u || output[2] != 99u)
            return false;
        ++cases;
    }

    std::fill(output, output + 3, 99u);
    written = 77;
    if (rustycore_forever_loaded_skill_values_order(nullptr, 1, output, 1, &written) != 1
        || written != 0 || output[0] != 99u)
        return false;
    ++cases;
    written = 77;
    if (rustycore_forever_loaded_skill_values_order(unique, 1, nullptr, 1, &written) != 1
        || written != 0)
        return false;
    ++cases;
    written = 77;
    if (rustycore_forever_loaded_skill_values_order(nullptr, 0, nullptr, 0, &written) != 0
        || written != 0)
        return false;
    ++cases;
    if (rustycore_forever_loaded_skill_values_order(nullptr, 0, nullptr, 0, nullptr) != 1)
        return false;
    ++cases;
    return true;
}
}

bool loaded_skill_values_source_oracle(std::size_t& cases)
{
    cases = 0;
    for (std::size_t length : {0, 1, 2, 12, 13, 14, 28, 29, 30, 58, 59, 60,
        126, 127, 128, 256, 257, 258, 299, 300, 301, 1000, 4096, 12000})
        for (unsigned mode = 0; mode != 5; ++mode)
        {
            if (!compare(length, mode))
                return false;
            ++cases;
        }
    if (!independent_lifecycles())
        return false;
    ++cases;
    return invalid_contract(cases);
}
