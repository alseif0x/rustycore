// 02245dcd Player.h:352; Player.cpp AddSpell/temporary/save membership operations.
// No persistent native Player book, payload mirror, reserve or rehash hint.
// The existing bridge translation unit enforces the pinned GNU toolchain.
#include "abi.hpp"
#include <unordered_map>

extern "C" int rustycore_forever_spell_book_order(
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
        std::unordered_map<std::uint32_t, std::uint8_t> entries;
        for (std::size_t i = 0; i < length; ++i)
        {
            auto const& mutation = mutations[i];
            switch (mutation.action)
            {
                case 1:
                    if (!entries.try_emplace(mutation.spell).second)
                        return 1;
                    break;
                case 2:
                {
                    auto entry = entries.find(mutation.spell);
                    if (entry == entries.end())
                        return 1;
                    entries.erase(entry);
                    break;
                }
                default:
                    return 1;
            }
        }
        if (entries.size() != capacity)
            return 1;
        // No further allocation/fallible work; output remains untouched on any
        // earlier validation failure. Payload size independence has an oracle.
        std::size_t at = 0;
        for (auto const& [id, unused] : entries)
            order[at++] = id;
        *written = at;
        return 0;
    }
    catch (...)
    {
        return 2;
    }
}
