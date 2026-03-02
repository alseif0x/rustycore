//! Custom collection types used throughout the WoW server.
//!
//! This crate provides:
//! - [`MultiMap`]: A map that stores multiple values per key using `SmallVec` for
//!   inline storage optimization.
//! - [`FlagArray`]: A compact bitfield array backed by `Vec<u32>`.

use smallvec::SmallVec;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// MultiMap
// ---------------------------------------------------------------------------

/// A hash-map that associates each key with a small vector of values.
///
/// The const generic `N` controls how many values are stored inline (on the
/// stack) per key before `SmallVec` spills to the heap.  The default is 4,
/// which is a good fit for the common WoW server case where most keys have
/// only a handful of values.
///
/// # Examples
///
/// ```
/// use wow_collections::MultiMap;
///
/// let mut mm = MultiMap::<&str, i32>::new();
/// mm.insert("a", 1);
/// mm.insert("a", 2);
/// mm.insert("b", 3);
///
/// assert_eq!(mm.get(&"a"), Some(&[1, 2][..]));
/// assert_eq!(mm.total_values(), 3);
/// ```
pub struct MultiMap<K, V, const N: usize = 4> {
    inner: HashMap<K, SmallVec<[V; N]>>,
}

impl<K, V, const N: usize> Default for MultiMap<K, V, N>
where
    K: Eq + std::hash::Hash,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V, const N: usize> MultiMap<K, V, N>
where
    K: Eq + std::hash::Hash,
{
    /// Creates an empty `MultiMap`.
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    /// Creates an empty `MultiMap` with at least the specified capacity for
    /// keys.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: HashMap::with_capacity(capacity),
        }
    }

    /// Inserts a value under the given key.  Multiple values can be stored
    /// under the same key.
    pub fn insert(&mut self, key: K, value: V) {
        self.inner.entry(key).or_default().push(value);
    }

    /// Returns the values associated with `key` as a slice, or `None` if the
    /// key is not present.
    pub fn get(&self, key: &K) -> Option<&[V]> {
        self.inner.get(key).map(|sv| sv.as_slice())
    }

    /// Returns a mutable reference to the `SmallVec` of values stored under
    /// `key`, or `None` if the key is not present.
    pub fn get_mut(&mut self, key: &K) -> Option<&mut SmallVec<[V; N]>> {
        self.inner.get_mut(key)
    }

    /// Returns `true` if the map contains the given key.
    pub fn contains_key(&self, key: &K) -> bool {
        self.inner.contains_key(key)
    }

    /// Removes a key and all its associated values, returning them if the key
    /// was present.
    pub fn remove(&mut self, key: &K) -> Option<SmallVec<[V; N]>> {
        self.inner.remove(key)
    }

    /// Removes a single value from the values stored under `key`.
    ///
    /// If the value is found it is removed (using swap-remove for O(1)
    /// performance).  If the key's value list becomes empty after removal, the
    /// key itself is also removed from the map.
    pub fn remove_value(&mut self, key: &K, value: &V)
    where
        V: PartialEq,
    {
        if let Some(values) = self.inner.get_mut(key) {
            if let Some(pos) = values.iter().position(|v| v == value) {
                values.swap_remove(pos);
            }
            if values.is_empty() {
                self.inner.remove(key);
            }
        }
    }

    /// Returns the number of keys in the map.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns the total number of values across all keys.
    pub fn total_values(&self) -> usize {
        self.inner.values().map(|sv| sv.len()).sum()
    }

    /// Returns `true` if the map contains no keys.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Removes all keys and values from the map.
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    /// Returns an iterator over the keys in the map.
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.inner.keys()
    }

    /// Returns an iterator yielding `(&K, &[V])` pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&K, &[V])> {
        self.inner.iter().map(|(k, v)| (k, v.as_slice()))
    }
}

impl<K, V, const N: usize> Clone for MultiMap<K, V, N>
where
    K: Eq + std::hash::Hash + Clone,
    V: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<K, V, const N: usize> std::fmt::Debug for MultiMap<K, V, N>
where
    K: Eq + std::hash::Hash + std::fmt::Debug,
    V: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MultiMap")
            .field("inner", &self.inner)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// FlagArray
// ---------------------------------------------------------------------------

/// A compact, dynamically-sized bitfield array backed by `Vec<u32>`.
///
/// Each bit corresponds to a flag that can be independently set, cleared, or
/// tested.  The size (in bits) is determined at construction time and the
/// backing storage is rounded up to the next multiple of 32.
///
/// # Examples
///
/// ```
/// use wow_collections::FlagArray;
///
/// let mut flags = FlagArray::new(100);
/// assert!(!flags.test(42));
/// flags.set(42);
/// assert!(flags.test(42));
/// assert_eq!(flags.count_set(), 1);
/// ```
#[derive(Clone, Debug)]
pub struct FlagArray {
    data: Vec<u32>,
}

impl FlagArray {
    /// Number of bits stored per element in the backing vector.
    const BITS_PER_WORD: usize = 32;

    /// Creates a new `FlagArray` with capacity for at least `size` bits, all
    /// initially cleared (zero).
    pub fn new(size: usize) -> Self {
        let word_count = size.div_ceil(Self::BITS_PER_WORD);
        Self {
            data: vec![0u32; word_count],
        }
    }

    /// Sets the bit at `index`.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of range (>= capacity in bits).
    pub fn set(&mut self, index: usize) {
        let (word, bit) = Self::position(index);
        assert!(word < self.data.len(), "FlagArray index {index} out of range");
        self.data[word] |= 1u32 << bit;
    }

    /// Clears the bit at `index`.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of range.
    pub fn clear(&mut self, index: usize) {
        let (word, bit) = Self::position(index);
        assert!(word < self.data.len(), "FlagArray index {index} out of range");
        self.data[word] &= !(1u32 << bit);
    }

    /// Returns `true` if the bit at `index` is set.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of range.
    pub fn test(&self, index: usize) -> bool {
        let (word, bit) = Self::position(index);
        assert!(word < self.data.len(), "FlagArray index {index} out of range");
        (self.data[word] & (1u32 << bit)) != 0
    }

    /// Clears all bits (sets every element to zero).
    pub fn reset_all(&mut self) {
        self.data.fill(0);
    }

    /// Returns `true` if any bit is set.
    pub fn any(&self) -> bool {
        self.data.iter().any(|&w| w != 0)
    }

    /// Returns `true` if no bits are set.
    pub fn none(&self) -> bool {
        !self.any()
    }

    /// Returns the total number of bits that are set.
    pub fn count_set(&self) -> usize {
        self.data.iter().map(|w| w.count_ones() as usize).sum()
    }

    /// Returns the total capacity of the flag array in bits.
    pub fn capacity(&self) -> usize {
        self.data.len() * Self::BITS_PER_WORD
    }

    /// Decomposes a bit index into (word_index, bit_position_within_word).
    #[inline]
    fn position(index: usize) -> (usize, usize) {
        (index / Self::BITS_PER_WORD, index % Self::BITS_PER_WORD)
    }
}

impl Default for FlagArray {
    /// Returns an empty `FlagArray` with zero capacity.
    fn default() -> Self {
        Self { data: Vec::new() }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---- MultiMap tests ---------------------------------------------------

    #[test]
    fn multimap_insert_and_get() {
        let mut mm = MultiMap::<&str, i32>::new();
        mm.insert("a", 1);
        mm.insert("a", 2);
        mm.insert("a", 3);
        mm.insert("b", 10);

        assert_eq!(mm.get(&"a"), Some(&[1, 2, 3][..]));
        assert_eq!(mm.get(&"b"), Some(&[10][..]));
        assert_eq!(mm.get(&"c"), None);
    }

    #[test]
    fn multimap_get_mut() {
        let mut mm = MultiMap::<i32, String>::new();
        mm.insert(1, "hello".to_string());
        mm.insert(1, "world".to_string());

        if let Some(values) = mm.get_mut(&1) {
            values.push("!".to_string());
        }
        assert_eq!(
            mm.get(&1),
            Some(&["hello".to_string(), "world".to_string(), "!".to_string()][..])
        );
    }

    #[test]
    fn multimap_contains_key() {
        let mut mm = MultiMap::<u32, u32>::new();
        assert!(!mm.contains_key(&1));
        mm.insert(1, 100);
        assert!(mm.contains_key(&1));
        assert!(!mm.contains_key(&2));
    }

    #[test]
    fn multimap_remove_key() {
        let mut mm = MultiMap::<&str, i32>::new();
        mm.insert("a", 1);
        mm.insert("a", 2);
        mm.insert("b", 3);

        let removed = mm.remove(&"a");
        assert!(removed.is_some());
        let removed = removed.unwrap();
        assert_eq!(removed.as_slice(), &[1, 2]);
        assert!(!mm.contains_key(&"a"));
        assert!(mm.contains_key(&"b"));
    }

    #[test]
    fn multimap_remove_nonexistent_key() {
        let mut mm = MultiMap::<&str, i32>::new();
        assert!(mm.remove(&"x").is_none());
    }

    #[test]
    fn multimap_remove_value() {
        let mut mm = MultiMap::<&str, i32>::new();
        mm.insert("a", 1);
        mm.insert("a", 2);
        mm.insert("a", 3);

        mm.remove_value(&"a", &2);
        // After swap_remove of index 1, order is [1, 3].
        let values = mm.get(&"a").unwrap();
        assert_eq!(values.len(), 2);
        assert!(values.contains(&1));
        assert!(values.contains(&3));
        assert!(!values.contains(&2));
    }

    #[test]
    fn multimap_remove_value_last_removes_key() {
        let mut mm = MultiMap::<&str, i32>::new();
        mm.insert("a", 1);

        mm.remove_value(&"a", &1);
        assert!(!mm.contains_key(&"a"));
        assert!(mm.is_empty());
    }

    #[test]
    fn multimap_remove_value_nonexistent() {
        let mut mm = MultiMap::<&str, i32>::new();
        mm.insert("a", 1);

        // Removing a value that doesn't exist should be a no-op.
        mm.remove_value(&"a", &999);
        assert_eq!(mm.get(&"a"), Some(&[1][..]));

        // Removing from a nonexistent key should be a no-op.
        mm.remove_value(&"b", &1);
    }

    #[test]
    fn multimap_len_and_total_values() {
        let mut mm = MultiMap::<&str, i32>::new();
        assert_eq!(mm.len(), 0);
        assert_eq!(mm.total_values(), 0);
        assert!(mm.is_empty());

        mm.insert("a", 1);
        mm.insert("a", 2);
        mm.insert("b", 3);
        mm.insert("c", 4);
        mm.insert("c", 5);
        mm.insert("c", 6);

        assert_eq!(mm.len(), 3);
        assert_eq!(mm.total_values(), 6);
        assert!(!mm.is_empty());
    }

    #[test]
    fn multimap_clear() {
        let mut mm = MultiMap::<i32, i32>::new();
        mm.insert(1, 10);
        mm.insert(2, 20);
        mm.clear();
        assert!(mm.is_empty());
        assert_eq!(mm.len(), 0);
        assert_eq!(mm.total_values(), 0);
    }

    #[test]
    fn multimap_keys() {
        let mut mm = MultiMap::<i32, i32>::new();
        mm.insert(1, 10);
        mm.insert(2, 20);
        mm.insert(3, 30);

        let mut keys: Vec<_> = mm.keys().copied().collect();
        keys.sort();
        assert_eq!(keys, vec![1, 2, 3]);
    }

    #[test]
    fn multimap_iter() {
        let mut mm = MultiMap::<i32, i32>::new();
        mm.insert(1, 10);
        mm.insert(1, 11);
        mm.insert(2, 20);

        let mut items: Vec<_> = mm.iter().map(|(&k, v)| (k, v.to_vec())).collect();
        items.sort_by_key(|(k, _)| *k);

        assert_eq!(items.len(), 2);
        assert_eq!(items[0], (1, vec![10, 11]));
        assert_eq!(items[1], (2, vec![20]));
    }

    #[test]
    fn multimap_default() {
        let mm: MultiMap<String, String> = MultiMap::default();
        assert!(mm.is_empty());
    }

    #[test]
    fn multimap_with_capacity() {
        let mm = MultiMap::<u32, u32>::with_capacity(100);
        assert!(mm.is_empty());
    }

    #[test]
    fn multimap_clone() {
        let mut mm = MultiMap::<i32, i32>::new();
        mm.insert(1, 10);
        mm.insert(1, 20);

        let mm2 = mm.clone();
        assert_eq!(mm2.get(&1), Some(&[10, 20][..]));
    }

    #[test]
    fn multimap_debug() {
        let mut mm = MultiMap::<i32, i32>::new();
        mm.insert(1, 10);
        let debug_str = format!("{:?}", mm);
        assert!(debug_str.contains("MultiMap"));
    }

    #[test]
    fn multimap_custom_n() {
        // Use N=1 to force spilling to heap quickly.
        let mut mm = MultiMap::<&str, i32, 1>::new();
        mm.insert("a", 1);
        mm.insert("a", 2);
        mm.insert("a", 3);
        assert_eq!(mm.get(&"a"), Some(&[1, 2, 3][..]));
        assert_eq!(mm.total_values(), 3);
    }

    // ---- FlagArray tests --------------------------------------------------

    #[test]
    fn flagarray_new_all_cleared() {
        let fa = FlagArray::new(100);
        assert!(fa.none());
        assert!(!fa.any());
        assert_eq!(fa.count_set(), 0);
    }

    #[test]
    fn flagarray_set_and_test() {
        let mut fa = FlagArray::new(128);
        fa.set(0);
        fa.set(31);
        fa.set(32);
        fa.set(63);
        fa.set(127);

        assert!(fa.test(0));
        assert!(fa.test(31));
        assert!(fa.test(32));
        assert!(fa.test(63));
        assert!(fa.test(127));

        // Bits that were not set should be false.
        assert!(!fa.test(1));
        assert!(!fa.test(30));
        assert!(!fa.test(64));
        assert!(!fa.test(126));
    }

    #[test]
    fn flagarray_clear_bit() {
        let mut fa = FlagArray::new(64);
        fa.set(10);
        assert!(fa.test(10));

        fa.clear(10);
        assert!(!fa.test(10));
    }

    #[test]
    fn flagarray_reset_all() {
        let mut fa = FlagArray::new(96);
        fa.set(0);
        fa.set(50);
        fa.set(95);
        assert_eq!(fa.count_set(), 3);

        fa.reset_all();
        assert!(fa.none());
        assert_eq!(fa.count_set(), 0);
    }

    #[test]
    fn flagarray_any_and_none() {
        let mut fa = FlagArray::new(32);
        assert!(fa.none());
        assert!(!fa.any());

        fa.set(15);
        assert!(fa.any());
        assert!(!fa.none());
    }

    #[test]
    fn flagarray_count_set() {
        let mut fa = FlagArray::new(256);
        for i in (0..256).step_by(2) {
            fa.set(i);
        }
        assert_eq!(fa.count_set(), 128);
    }

    #[test]
    fn flagarray_capacity() {
        // 100 bits -> ceil(100/32) = 4 words -> 128 bits capacity.
        let fa = FlagArray::new(100);
        assert_eq!(fa.capacity(), 128);

        // Exact multiple of 32.
        let fa = FlagArray::new(64);
        assert_eq!(fa.capacity(), 64);
    }

    #[test]
    fn flagarray_zero_size() {
        let fa = FlagArray::new(0);
        assert!(fa.none());
        assert_eq!(fa.count_set(), 0);
        assert_eq!(fa.capacity(), 0);
    }

    #[test]
    fn flagarray_single_bit() {
        let mut fa = FlagArray::new(1);
        assert_eq!(fa.capacity(), 32);
        assert!(!fa.test(0));

        fa.set(0);
        assert!(fa.test(0));
        assert_eq!(fa.count_set(), 1);
    }

    #[test]
    fn flagarray_boundary_bits() {
        // Test bits right at the boundary of each u32 word.
        let mut fa = FlagArray::new(128);
        let boundary_bits = [0, 31, 32, 63, 64, 95, 96, 127];
        for &bit in &boundary_bits {
            fa.set(bit);
        }
        assert_eq!(fa.count_set(), boundary_bits.len());
        for &bit in &boundary_bits {
            assert!(fa.test(bit));
        }
    }

    #[test]
    fn flagarray_set_clear_idempotent() {
        let mut fa = FlagArray::new(64);

        // Setting the same bit twice should have no additional effect.
        fa.set(10);
        fa.set(10);
        assert_eq!(fa.count_set(), 1);

        // Clearing a bit that's already clear should be harmless.
        fa.clear(20);
        assert_eq!(fa.count_set(), 1);
    }

    #[test]
    fn flagarray_default() {
        let fa = FlagArray::default();
        assert!(fa.none());
        assert_eq!(fa.capacity(), 0);
    }

    #[test]
    fn flagarray_clone() {
        let mut fa = FlagArray::new(64);
        fa.set(10);
        fa.set(50);

        let fa2 = fa.clone();
        assert!(fa2.test(10));
        assert!(fa2.test(50));
        assert_eq!(fa2.count_set(), 2);
    }

    #[test]
    fn flagarray_debug() {
        let fa = FlagArray::new(32);
        let debug_str = format!("{:?}", fa);
        assert!(debug_str.contains("FlagArray"));
    }

    #[test]
    #[should_panic(expected = "out of range")]
    fn flagarray_set_out_of_range() {
        let mut fa = FlagArray::new(32);
        fa.set(32); // capacity is 32 bits (indices 0..31), so 32 is out of range.
    }

    #[test]
    #[should_panic(expected = "out of range")]
    fn flagarray_test_out_of_range() {
        let fa = FlagArray::new(32);
        fa.test(32);
    }

    #[test]
    #[should_panic(expected = "out of range")]
    fn flagarray_clear_out_of_range() {
        let mut fa = FlagArray::new(32);
        fa.clear(32);
    }

    #[test]
    fn flagarray_large() {
        // Test with a large flag array to ensure no issues at scale.
        let mut fa = FlagArray::new(10_000);
        for i in 0..10_000 {
            if i % 3 == 0 {
                fa.set(i);
            }
        }
        // Numbers 0..9999 where i % 3 == 0: 0, 3, 6, ... 9999
        // Count = ceil(10000 / 3) = 3334
        assert_eq!(fa.count_set(), 3334);
        assert!(fa.test(0));
        assert!(!fa.test(1));
        assert!(!fa.test(2));
        assert!(fa.test(3));
        assert!(fa.test(9999));
    }
}
