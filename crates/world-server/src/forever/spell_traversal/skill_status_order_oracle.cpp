// Actual SkillStatusMap/SkillStatusData/SkillUpdateState declarations are
// extracted from immutable 02245dcd Player.h:687-704 by the QA driver.
// This builds the source container independently; no replay internals imported.
#include "abi.hpp"
#include "SourceSkillStatusMap.hpp"
#include <algorithm>
#include <limits>
#include <vector>

namespace
{
bool attempt(SkillStatusMap& source, std::vector<std::uint32_t>& history,
    std::uint32_t id, std::uint32_t position, SkillUpdateState state)
{
    auto [entry, inserted] = source.try_emplace(id, position, state);
    if (inserted)
        history.push_back(id);
    return inserted;
}

bool matches_source(SkillStatusMap const& source, std::vector<std::uint32_t> const& history)
{
    std::vector<std::uint32_t> expected;
    for (auto const& [id, payload] : source)
        expected.push_back(id); // Unchanged/Deleted remain physical members.
    std::vector<std::uint32_t> output(source.size(), 99);
    std::size_t written = 77;
    return rustycore_forever_skill_status_order(history.data(), history.size(),
        output.data(), output.size(), &written) == 0
        && written == expected.size() && output == expected;
}

bool compare(std::size_t length, unsigned keyMode, bool mutatePayload)
{
    SkillStatusMap source;
    std::vector<std::uint32_t> history;
    std::vector<std::uint32_t> ids;
    for (std::size_t i = 0; i < length; ++i)
    {
        auto value = static_cast<std::uint32_t>(i);
        ids.push_back(keyMode == 0 ? value : keyMode == 1 ? value * 131u
            : keyMode == 2 ? value * 65536u
            : keyMode == 3 ? std::numeric_limits<std::uint32_t>::max() - value
            : 0x80000000u ^ (value * 257u));
    }
    if (keyMode % 2)
        std::reverse(ids.begin(), ids.end());
    constexpr SkillUpdateState states[] = {
        SKILL_UNCHANGED, SKILL_CHANGED, SKILL_NEW, SKILL_DELETED};
    for (std::size_t i = 0; i < ids.size(); ++i)
    {
        auto id = ids[i];
        auto position = static_cast<std::uint32_t>(i % 300);
        if (!attempt(source, history, id, position, SKILL_UNCHANGED))
            return false;
        auto const count = history.size();
        // A duplicate attempt must preserve the original constructor payload.
        if (attempt(source, history, id, 299, SKILL_DELETED)
            || source.at(id).pos != position || source.at(id).uState != SKILL_UNCHANGED
            || history.size() != count)
            return false;
        if (mutatePayload)
        {
            auto& payload = source.at(id);
            payload.pos = static_cast<std::uint32_t>((i + 11) % 300);
            payload.uState = states[i % 4];
            auto const changedPosition = payload.pos;
            auto const changedState = payload.uState;
            if (attempt(source, history, id, 0, SKILL_NEW)
                || payload.pos != changedPosition || payload.uState != changedState
                || history.size() != count)
                return false;
        }
    }
    return history.size() == source.size() && matches_source(source, history);
}

bool captured_slot_alias(std::size_t& cases)
{
    SkillStatusMap source;
    std::vector<std::uint32_t> history;
    if (!attempt(source, history, 1, 0, SKILL_UNCHANGED)
        || !attempt(source, history, 20, 1, SKILL_NEW)
        || !attempt(source, history, 10, 1, SKILL_NEW)
        || history != std::vector<std::uint32_t>{1, 20, 10}
        || source.at(20).pos != 1 || source.at(10).pos != 1
        || !matches_source(source, history))
        return false;
    ++cases;
    source.at(20).uState = SKILL_DELETED;
    source.at(10).uState = SKILL_CHANGED;
    if (attempt(source, history, 20, 2, SKILL_UNCHANGED)
        || source.at(20).pos != 1 || source.at(20).uState != SKILL_DELETED
        || !matches_source(source, history))
        return false;
    ++cases;
    return true;
}

bool independent_lifecycles(std::size_t& cases)
{
    SkillStatusMap first;
    std::vector<std::uint32_t> firstHistory;
    for (auto id : {0u, 65536u, std::numeric_limits<std::uint32_t>::max()})
        if (!attempt(first, firstHistory, id, 1, SKILL_UNCHANGED))
            return false;
    if (!matches_source(first, firstHistory))
        return false;
    ++cases;
    {
        SkillStatusMap second;
        std::vector<std::uint32_t> secondHistory;
        if (!attempt(second, secondHistory, 0, 0, SKILL_NEW)
            || !attempt(second, secondHistory, 1, 1, SKILL_DELETED)
            || !matches_source(second, secondHistory))
            return false;
        ++cases;
    }
    SkillStatusMap empty;
    if (!matches_source(empty, {}) || !matches_source(first, firstHistory))
        return false;
    cases += 2;
    return true;
}
}

bool skill_status_source_oracle(std::size_t& cases)
{
    cases = 0;
    // Rehash-boundary neighborhoods plus >300 and large numeric stress inputs;
    // these synthetic keys are container evidence, not DB2/Player admission.
    for (std::size_t length : {0, 1, 2, 12, 13, 14, 28, 29, 30, 58, 59, 60,
        126, 127, 128, 256, 257, 258, 299, 300, 301, 1000, 4096, 12000})
        for (unsigned mode = 0; mode != 5; ++mode)
            for (bool mutate : {false, true})
            {
                if (!compare(length, mode, mutate))
                    return false;
                ++cases;
            }
    return captured_slot_alias(cases) && independent_lifecycles(cases);
}
