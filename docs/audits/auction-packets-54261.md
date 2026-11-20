# Auction packet evidence for client 3.4.3.54261

Reviewed on 2026-11-20 for PR #1494. This contract belongs to the `3.4.3`
version line. The Forever 1.60.1.70170 protocol requires its own evidence.

## Client provenance

The executable and initialized text image are the same verified inputs recorded
in [the InspectResult evidence](inspect-result-54261.md):

- `WowClassic.exe`, 50,589,832 bytes, SHA-256
  `e240f9d88445643d2545ac54b3c3874b2f86336bc8947256584ef07bb5fd66a7`.
- Preferred image base `0x140000000`; text starts at `0x140001000`.
- Initialized text dump SHA-256
  `fcd5f3c695d3f6b5812d0535cd704fa7ee1893e3630ab5951a7cd80c6ad6058b`.

The maintainer inspected these existing private bytes offline. No new account
login, game-server operation or auction transaction was performed. The client
executable and text image are not distributed with this repository.

## AuctionHelloResponse

The opcode binding is established independently of a structural resemblance:

1. The server-message dispatcher at `0x140727d90` takes the opcode in `r9w`,
   subtracts `0x256c`, bounds the index at `0x340`, and indexes the RVA table
   at `0x14073651c`.
2. `SMSG_AUCTION_HELLO_RESPONSE`, opcode `0x26ee`, selects index `0x182`.
   Its table entry at `0x140736b24` contains RVA `0x72f0e4`.
3. That case calls the message reader at `0x1406b7c30`, at
   `0x14072f0f9`, then dispatches the constructed message through the callback
   slot at `0x14309f278`.
4. The reader calls the packed-GUID reader at `0x141550b20` from
   `0x1406b7c81`. It then calls the four-byte reader `0x1402c9a30` at
   `0x1406b7c92`, `0x1406b7caa` and `0x1406b7cc2`, storing the results at
   object offsets `0x34`, `0x38` and `0x3c`.
5. At `0x1406b7cdb` it reads one byte with `0x1402c9990`, shifts its high bit
   at `0x1406b7cef`, and stores that flag at object offset `0x30`.

The supported wire order is therefore packed GUID, purchased-delivery delay,
cancelled-delivery delay, auction-house ID, and the OpenForBusiness bit in its
own final byte. The pinned server writer at
`AuctionHousePackets.cpp:588-598` omits the third `u32`; that inherited writer
does not establish this client's layout.

## AuctionRemoveItem and packed GUIDs

The client writer at `0x14075bf30` writes opcode `0x34cc` at `0x14075bf48`,
then writes the packed GUID at `0x14075bf54` through `0x141550bf0`. It writes
one `u32` from object offset `0x60` at `0x14075bf5f`, through `0x1402ca480`.
The following code writes the optional-addon presence bit at `0x14075bf8b`
and conditionally writes the addon at `0x14075bf9d`. There is no second item-ID
word between the auction ID and the presence bit.

The contributor supplied the 14-byte cancellation fixture in
[`bb944a3b`](https://github.com/alfishe/rustycore/commit/bb944a3b54e0194c0cf06b66f59fb23500411dc1).
Its two masks select seven GUID bytes: nine bytes for the packed GUID, four
for auction ID 1, and one final zero presence byte. It is retained as a
contributor-provided capture fixture, rather than attributed to a new maintainer
capture.

The shared client GUID reader at `0x141550b20` reads the low and high masks
first, then reconstructs each half from its selected bytes. The writer at
`0x141550bf0` reserves both masks before emitting the two halves. This agrees
with `ObjectGuid.cpp:779-785` and the GUID operators used by the pinned
`AuctionPlaceBid`, `AuctionReplicateItems`, `AuctionSellItem` and item-entry
readers in `AuctionHousePackets.cpp`.

## Source identity for the auction house

The house selector follows `AuctionHouseMgr::GetAuctionHouseEntry`,
`AuctionHouseMgr.cpp:806-842`, at pinned source
`a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`:

- Cross-faction auctions select shared house 1.
- Otherwise faction templates 120, 474 and 855 select goblin house 7.
- Otherwise the Alliance faction-group bit selects house 2, before the Horde
  bit selects house 6. Unknown and other groups select house 1.

The option is `AllowTwoSide.Interaction.Auction`, default true in the pinned
`World.cpp:915`. An explicit false value restores the per-faction selection. Packet
construction requires the selected ID explicitly; a neutral-house constant is
not a substitute for the validated NPC's identity and session policy.

This evidence establishes the bounded packet layouts and house-selection
contract. It does not establish live auction UI acceptance, completed auction
transactions or the unimplemented listing/cancellation backend.

## Maintainer integration review

The original contribution is replayed as one commit on `3.4.3`; its Forever
ancestry is excluded. All three Hello producers use the validated creature's
faction template and the existing immutable faction-template catalog. Startup
passes the already-parsed auction policy into the existing session configuration;
no new database access, mutable owner, lock, task or clock is introduced.

The R5 comparison against the replayed contribution changes production methods
from 1,239 to 1,240, distinct method names from 1,238 to 1,239, and production
lines from 34,142 to 34,145. The sole new `WorldSession` method is the externally
used startup configuration setter. Fields, thunks, impl files and test shims are
unchanged. The four reviewed persistence snapshot changes are fingerprints of
the same `SessionResources` initializer with one additional immutable option;
their database targets, classifications and multiplicities are unchanged, and
the derived persistence boundary policy is identical.

Regression coverage includes the supplied cancellation bytes with the exact
auctioneer GUID, optional-addon truncation, the Hello bit byte and delivery
delays, faction/config precedence, all three production Hello entry points,
and silent rejection of malformed or missing-NPC requests and GameObject
auction options. This is static and automated evidence, not a live UI claim.
