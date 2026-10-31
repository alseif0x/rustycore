// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: bounded, atomic preparation and consumer acquisition.
use super::*;

fn admit(state: &mut State, limit: usize, base: usize, extra: usize) -> Result<(), Error> {
    let total = base.checked_add(extra).ok_or(Error::Limits)?;
    if total > limit {
        return Err(Error::Limits);
    }
    state.admitted_peak_bytes = state.admitted_peak_bytes.max(total);
    Ok(())
}

impl ForeverTerrain {
    /// Prepare outside world locks. A failed preparation retains prior geometry
    /// and leases; observed source changes instead poison all future queries.
    /// The existing GridMap parser uses infallible fixed-size array allocations;
    /// the transient allowance bounds admitted bytes, not allocator/OOM behavior.
    pub fn prepare(self: &Arc<Self>, key: TileKey, ground: Ground<'_>) -> Result<TileLease, Error> {
        let mut state = self.state.write().map_err(|_| Error::LockPoisoned)?;
        state.healthy(key.map)?;
        let index = match state.position(key) {
            Ok(index) => {
                if !state.tiles[index].same_input(ground) {
                    state.input_changed = true;
                    return Err(Error::InputChanged);
                }
                if state.leases >= self.limits.max_leases {
                    return Err(Error::Limits);
                }
                state.store.verify_source()?;
                state.tiles[index].consumers += 1;
                state.leases += 1;
                return Ok(TileLease {
                    owner: Arc::clone(self),
                    key,
                });
            }
            Err(index) => index,
        };
        if state.tiles.len() >= self.limits.max_tiles || state.leases >= self.limits.max_leases {
            return Err(Error::Limits);
        }
        if let Ground::Complete(bytes) = ground {
            forever_terrain::maps::verify(
                bytes,
                self.profile.build,
                forever_terrain::maps::MAX_MAP_BYTES,
            )
            .map_err(|_| Error::GroundFormat)?;
        }

        let base = state.resident_bytes();
        let limit = self.limits.max_terrain_bytes;
        // Keep old registry alive until every fallible step succeeds. Account
        // both capacities during growth; moved Tile values never clone geometry.
        let mut replacement = None;
        let mut extra = 0;
        if state.tiles.len() == state.tiles.capacity() {
            let desired = state
                .tiles
                .len()
                .saturating_mul(2)
                .max(1)
                .min(self.limits.max_tiles);
            admit(&mut state, limit, base, desired * size_of::<Tile>())?;
            let mut registry = Vec::<Tile>::new();
            registry
                .try_reserve_exact(desired)
                .map_err(|_| Error::Allocation)?;
            extra = registry.capacity() * size_of::<Tile>();
            admit(&mut state, limit, base, extra)?;
            replacement = Some(registry);
        }
        let (bytes, parsed) = match ground {
            Ground::Absent => (None, None),
            Ground::Complete(input) => {
                admit(
                    &mut state,
                    limit,
                    base,
                    extra + input.len() + GRID_PARSE_ALLOWANCE,
                )?;
                let mut owned = Vec::new();
                owned
                    .try_reserve_exact(input.len())
                    .map_err(|_| Error::Allocation)?;
                admit(
                    &mut state,
                    limit,
                    base,
                    extra + owned.capacity() + GRID_PARSE_ALLOWANCE,
                )?;
                owned.extend_from_slice(input);
                let grid = GridMap::parse(&owned).ok_or(Error::GroundFormat)?;
                if grid.heap_bytes() > GRID_PARSE_ALLOWANCE {
                    return Err(Error::Limits);
                }
                (Some(owned), Some(grid))
            }
        };
        let options = state.store.options();
        let collision_loaded = options.enable_height || options.enable_line_of_sight;
        if collision_loaded {
            state.store.load_tile(key.map, u16::from(key.x), key.y)?;
        } else {
            state.store.verify_source()?;
        }
        // Commit: capacity was reserved above; no fallible allocation or I/O.
        if let Some(mut registry) = replacement {
            registry.append(&mut state.tiles);
            state.tiles = registry;
        }
        state.tiles.insert(
            index,
            Tile {
                key,
                consumers: 1,
                bytes,
                ground: parsed,
                collision_loaded,
            },
        );
        state.leases += 1;
        Ok(TileLease {
            owner: Arc::clone(self),
            key,
        })
    }
}
