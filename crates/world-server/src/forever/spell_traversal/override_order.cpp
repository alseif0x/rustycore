// 02245dcd Player.h:3272 / Player.cpp:31011-31024.
// Numeric temporary replay, never a native Player/override membership owner.
#include "abi.hpp"
#include <unordered_set>

extern "C" int rustycore_forever_spell_override_order(
    ForeverSpellBookMutation const* mutations, std::size_t length,
    std::uint32_t* order, std::size_t capacity, std::size_t* written) noexcept
{
    if (!written)
        return 1;
    *written = 0;
    if ((length && !mutations) || (capacity && !order))
        return 1;
    try
    {
        std::unordered_set<std::uint32_t> entries;
        for (std::size_t i = 0; i < length; ++i)
        {
            auto const& mutation = mutations[i];
            switch (mutation.action)
            {
                case 1:
                    if (!entries.insert(mutation.spell).second)
                        return 1;
                    break;
                case 2:
                    if (entries.erase(mutation.spell) != 1)
                        return 1;
                    break;
                default:
                    return 1;
            }
        }
        if (entries.size() != capacity)
            return 1;
        // All validation/allocation finished before caller-visible output.
        std::size_t at = 0;
        for (std::uint32_t id : entries)
            order[at++] = id;
        *written = at;
        return 0;
    }
    catch (...)
    {
        return 2;
    }
}
