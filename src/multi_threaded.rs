use crate::{
    arena::Arena,
    hasher::RandomState,
    key::{Key, Spur},
    reader::RodeoReader,
    resolver::RodeoResolver,
    Capacity, MemoryLimits,
};
use core::{
    fmt::{Debug, Formatter, Result as FmtResult},
    hash::{BuildHasher, Hash, Hasher},
    iter, mem,
    sync::atomic::{AtomicUsize, Ordering},
};
use dashmap::DashMap;
use hashbrown::{hash_map::RawEntryMut, HashMap};
use std::sync::Mutex;

#[cfg(debug_assertions)]
macro_rules! index_unchecked_mut {
    ($slice:expr, $idx:expr) => {{
        // Keeps unsafe required even when debug assertions are off
        unsafe fn x() {}
        x();

        let elem: &mut _ = &mut $slice[$idx];
        elem
    }};
}

#[cfg(not(debug_assertions))]
macro_rules! index_unchecked_mut {
    ($slice:expr, $idx:expr) => {{
        let elem: &mut _ = $slice.get_unchecked_mut($idx);
        elem
    }};
}

/// A concurrent string interner that caches strings quickly with a minimal memory footprint,
/// returning a unique key to re-access it with `O(1)` internment and resolution.
///
/// This struct is only avaliable with the `multi-threaded` feature!
/// By default ThreadedRodeo uses the [`Spur`] type for keys and [`RandomState`] as the hasher
///
/// [`Spur`]: crate::Spur
/// [`ahash::RandomState`]: https://docs.rs/ahash/0.3.2/ahash/struct.RandomState.html
/// [`RandomState`]: index.html#cargo-features
pub struct ThreadedRodeo<K = Spur, S = RandomState> {
    // TODO: Should this be migrated over to the scheme that `Rodeo` uses for string storage?
    //       Need benchmarks to see the perf impact of two dashmap lookups and see if that's worth
    //       the storage impact of extra string pointers lying around
    /// Map that allows str to key resolution
    map: DashMap<&'static str, K, S>,
    /// Map that allows key to str resolution
    pub(crate) strings: DashMap<K, &'static str, S>,
    /// The current key value
    key: AtomicUsize,
    /// The arena where all strings are stored
    arena: Mutex<Arena>,
}

// TODO: More parity functions with std::HashMap

impl<K> ThreadedRodeo<K, RandomState>
where
    K: Key + Hash,
{
    /// Create a new ThreadedRodeo
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::{ThreadedRodeo, Spur};
    /// use std::{thread, sync::Arc};
    ///
    /// let lasso: Arc<ThreadedRodeo<Spur>> = Arc::new(ThreadedRodeo::new());
    /// let hello = lasso.get_or_intern("Hello, ");
    ///
    /// let l = Arc::clone(&lasso);
    /// let world = thread::spawn(move || {
    ///     l.get_or_intern("World!")
    /// })
    /// .join()
    /// .unwrap();
    ///
    /// assert_eq!("Hello, ", lasso.resolve(&hello));
    /// assert_eq!("World!", lasso.resolve(&world));
    /// ```
    ///
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn new() -> Self {
        Self::with_capacity_memory_limits_and_hasher(
            Capacity::default(),
            MemoryLimits::default(),
            RandomState::new(),
        )
    }

    /// Create a new ThreadedRodeo with the specified capacity. The interner will be able to hold `capacity`
    /// strings without reallocating. If capacity is 0, the interner will not allocate.
    ///
    /// See [`Capacity`] for more details
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::{ThreadedRodeo, Capacity, Spur};
    ///
    /// let rodeo: ThreadedRodeo<Spur> = ThreadedRodeo::with_capacity(Capacity::for_strings(10));
    /// ```
    ///
    /// [`Capacity`]: crate::Capacity
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn with_capacity(capacity: Capacity) -> Self {
        Self::with_capacity_memory_limits_and_hasher(
            capacity,
            MemoryLimits::default(),
            RandomState::new(),
        )
    }

    /// Create a new ThreadedRodeo with the specified memory limits. The interner will be able to hold `max_memory_usage`
    /// bytes of interned strings until it will start returning `None` from `try_get_or_intern` or panicking from
    /// `get_or_intern`.
    ///
    /// Note: If the capacity of the interner is greater than the memory limit, then that will be the effective maximum
    /// for allocated memory
    ///
    /// See [`MemoryLimits`] for more information
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::{ThreadedRodeo, MemoryLimits, Spur};
    ///
    /// let rodeo: ThreadedRodeo<Spur> = ThreadedRodeo::with_memory_limits(MemoryLimits::for_memory_usage(4096));
    /// ```
    ///
    /// [`MemoryLimits`]: crate::MemoryLimits
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn with_memory_limits(memory_limits: MemoryLimits) -> Self {
        Self::with_capacity_memory_limits_and_hasher(
            Capacity::default(),
            memory_limits,
            RandomState::new(),
        )
    }

    /// Create a new ThreadedRodeo with the specified capacity and memory limits. The interner will be able to hold `max_memory_usage`
    /// bytes of interned strings until it will start returning `None` from `try_get_or_intern` or panicking from
    /// `get_or_intern`.
    ///
    /// Note: If the capacity of the interner is greater than the memory limit, then that will be the effective maximum
    /// for allocated memory
    ///
    /// See [`Capacity`] [`MemoryLimits`] for more information
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::{ThreadedRodeo, MemoryLimits, Spur};
    ///
    /// let rodeo: ThreadedRodeo<Spur> = ThreadedRodeo::with_memory_limits(MemoryLimits::for_memory_usage(4096));
    /// ```
    ///
    /// [`Capacity`]: crate::Capacity
    /// [`MemoryLimits`]: crate::MemoryLimits
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn with_capacity_and_memory_limits(
        capacity: Capacity,
        memory_limits: MemoryLimits,
    ) -> Self {
        Self::with_capacity_memory_limits_and_hasher(capacity, memory_limits, RandomState::new())
    }
}

impl<K, S> ThreadedRodeo<K, S>
where
    K: Key + Hash,
    S: BuildHasher + Clone,
{
    /// Creates an empty ThreadedRodeo which will use the given hasher for its internal hashmap
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::{Spur, ThreadedRodeo};
    /// use std::collections::hash_map::RandomState;
    ///
    /// let rodeo: ThreadedRodeo<Spur, RandomState> = ThreadedRodeo::with_hasher(RandomState::new());
    /// ```
    ///
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn with_hasher(hash_builder: S) -> Self {
        Self::with_capacity_memory_limits_and_hasher(
            Capacity::default(),
            MemoryLimits::default(),
            hash_builder,
        )
    }

    /// Creates a new ThreadedRodeo with the specified capacity that will use the given hasher for its internal hashmap
    ///
    /// See [`Capacity`] for more details
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::{Spur, Capacity, ThreadedRodeo};
    /// use std::collections::hash_map::RandomState;
    ///
    /// let rodeo: ThreadedRodeo<Spur, RandomState> = ThreadedRodeo::with_capacity_and_hasher(Capacity::for_strings(10), RandomState::new());
    /// ```
    ///
    /// [`Capacity`]: crate::Capacity
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn with_capacity_and_hasher(capacity: Capacity, hash_builder: S) -> Self {
        Self::with_capacity_memory_limits_and_hasher(
            capacity,
            MemoryLimits::default(),
            hash_builder,
        )
    }

    /// Creates a new ThreadedRodeo with the specified capacity and memory limits that will use the given hasher for its internal hashmap
    ///
    /// See [`Capacity`] and [`MemoryLimits`] for more information
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::{Spur, Capacity, MemoryLimits, ThreadedRodeo};
    /// use std::collections::hash_map::RandomState;
    ///
    /// let rodeo: ThreadedRodeo<Spur, RandomState> = ThreadedRodeo::with_capacity_memory_limits_and_hasher(
    ///     Capacity::for_strings(10),
    ///     MemoryLimits::for_memory_usage(4096),
    ///     RandomState::new(),
    /// );
    /// ```
    ///
    /// [`Capacity`]: crate::Capacity
    /// [`MemoryLimits`]: crate::MemoryLimits
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn with_capacity_memory_limits_and_hasher(
        capacity: Capacity,
        memory_limits: MemoryLimits,
        hash_builder: S,
    ) -> Self {
        let Capacity { strings, bytes } = capacity;
        let MemoryLimits { max_memory_usage } = memory_limits;

        Self {
            map: DashMap::with_capacity_and_hasher(strings, hash_builder.clone()),
            strings: DashMap::with_capacity_and_hasher(strings, hash_builder),
            key: AtomicUsize::new(0),
            arena: Mutex::new(Arena::new(bytes, max_memory_usage)),
        }
    }

    /// Get the key for a string, interning it if it does not yet exist
    ///
    /// # Panics
    ///
    /// Panics if the key's `try_from_usize` function fails. With the default keys, this means that
    /// you've interned more strings than it can handle. (For [`Spur`] this means that `u32::MAX - 1`
    /// unique strings were interned)
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::ThreadedRodeo;
    ///
    /// let rodeo = ThreadedRodeo::default();
    ///
    /// // Interned the string
    /// let key = rodeo.get_or_intern("Strings of things with wings and dings");
    /// assert_eq!("Strings of things with wings and dings", rodeo.resolve(&key));
    ///
    /// // No string was interned, as it was already contained
    /// let key = rodeo.get_or_intern("Strings of things with wings and dings");
    /// assert_eq!("Strings of things with wings and dings", rodeo.resolve(&key));
    /// ```
    ///
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn get_or_intern<T>(&self, val: T) -> K
    where
        T: AsRef<str>,
    {
        self.try_get_or_intern(val)
            .expect("Failed to get or intern string")
    }

    /// Get the key for a string, interning it if it does not yet exist
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::ThreadedRodeo;
    ///
    /// let rodeo = ThreadedRodeo::default();
    ///
    /// // Interned the string
    /// let key = rodeo.get_or_intern("Strings of things with wings and dings");
    /// assert_eq!("Strings of things with wings and dings", rodeo.resolve(&key));
    ///
    /// // No string was interned, as it was already contained
    /// let key = rodeo.get_or_intern("Strings of things with wings and dings");
    /// assert_eq!("Strings of things with wings and dings", rodeo.resolve(&key));
    /// ```
    ///
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn try_get_or_intern<T>(&self, val: T) -> Option<K>
    where
        T: AsRef<str>,
    {
        let string_slice = val.as_ref();

        if let Some(key) = self.map.get(string_slice) {
            Some(*key)
        } else {
            let shard = self.map.determine_map(string_slice);
            // Safety: The indices provided by DashMap always refer to a shard in it's shards
            let shard = unsafe { self.map.shards().get_unchecked(shard) };

            if let Some(key) = shard.read().get(string_slice) {
                return Some(*key.get());
            }

            // Safety: The drop impl removes all references before the arena is dropped
            let string: &'static str =
                unsafe { self.arena.lock().unwrap().store_str(string_slice)? };
            let key = K::try_from_usize(self.key.fetch_add(1, Ordering::SeqCst))?;

            self.map.insert(string, key);
            self.strings.insert(key, string);

            Some(key)
        }
    }

    /// Get the key for a static string, interning it if it does not yet exist
    ///
    /// This will not reallocate or copy the given string but will instead just store it
    ///
    /// # Panics
    ///
    /// Panics if the key's `try_from_usize` function fails. With the default keys, this means that
    /// you've interned more strings than it can handle. (For [`Spur`] this means that `u32::MAX - 1`
    /// unique strings were interned)
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::ThreadedRodeo;
    ///
    /// let mut rodeo = ThreadedRodeo::default();
    ///
    /// // Interned the string
    /// let key = rodeo.get_or_intern_static("Strings of things with wings and dings");
    /// assert_eq!("Strings of things with wings and dings", rodeo.resolve(&key));
    ///
    /// // No string was interned, as it was already contained
    /// let key = rodeo.get_or_intern_static("Strings of things with wings and dings");
    /// assert_eq!("Strings of things with wings and dings", rodeo.resolve(&key));
    /// ```
    ///
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn get_or_intern_static(&self, string: &'static str) -> K {
        self.try_get_or_intern_static(string)
            .expect("Failed to get or intern static string")
    }

    /// Get the key for a static string, interning it if it does not yet exist
    ///
    /// This will not reallocate and copy the given string but will instead just store it
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::ThreadedRodeo;
    ///
    /// let mut rodeo = ThreadedRodeo::default();
    ///
    /// // Interned the string
    /// let key = rodeo.try_get_or_intern_static("Strings of things with wings and dings").unwrap();
    /// assert_eq!("Strings of things with wings and dings", rodeo.resolve(&key));
    ///
    /// // No string was interned, as it was already contained
    /// let key = rodeo.try_get_or_intern_static("Strings of things with wings and dings").unwrap();
    /// assert_eq!("Strings of things with wings and dings", rodeo.resolve(&key));
    /// ```
    ///
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn try_get_or_intern_static(&self, string: &'static str) -> Option<K> {
        if let Some(key) = self.map.get(string) {
            Some(*key)
        } else {
            let shard = self.map.determine_map(string);
            // Safety: The indices provided by DashMap always refer to a shard in it's shards
            let shard = unsafe { self.map.shards().get_unchecked(shard) };

            if let Some(key) = shard.read().get(string) {
                return Some(*key.get());
            }

            let key = K::try_from_usize(self.key.fetch_add(1, Ordering::SeqCst))?;

            self.map.insert(string, key);
            self.strings.insert(key, string);

            Some(key)
        }
    }

    /// Get the key value of a string, returning `None` if it doesn't exist
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::ThreadedRodeo;
    ///
    /// let rodeo = ThreadedRodeo::default();
    ///
    /// let key = rodeo.get_or_intern("Strings of things with wings and dings");
    /// assert_eq!(Some(key), rodeo.get("Strings of things with wings and dings"));
    ///
    /// assert_eq!(None, rodeo.get("This string isn't interned"));
    /// ```
    ///
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn get<T>(&self, val: T) -> Option<K>
    where
        T: AsRef<str>,
    {
        self.map.get(val.as_ref()).map(|k| *k)
    }

    /// Resolves a string by its key. Only keys made by the current ThreadedRodeo may be used
    ///
    /// # Panics
    ///
    /// Panics if the key is out of bounds
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::ThreadedRodeo;
    ///
    /// let rodeo = ThreadedRodeo::default();
    ///
    /// let key = rodeo.get_or_intern("Strings of things with wings and dings");
    /// assert_eq!("Strings of things with wings and dings", rodeo.resolve(&key));
    /// ```
    ///
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn resolve<'a>(&'a self, key: &K) -> &'a str {
        &*self.strings.get(key).expect("Key out of bounds")
    }

    /// Resolves a string by its key, returning `None` if it is out of bounds. Only keys made by the current
    /// ThreadedRodeo may be used
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::ThreadedRodeo;
    ///
    /// let rodeo = ThreadedRodeo::default();
    ///
    /// let key = rodeo.get_or_intern("Strings of things with wings and dings");
    /// assert_eq!(Some("Strings of things with wings and dings"), rodeo.try_resolve(&key));
    /// ```
    ///
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn try_resolve<'a>(&'a self, key: &K) -> Option<&'a str> {
        self.strings.get(key).map(|s| *s)
    }

    /// Gets the number of interned strings
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::ThreadedRodeo;
    ///
    /// let rodeo = ThreadedRodeo::default();
    /// rodeo.get_or_intern("Documentation often has little hidden bits in it");
    ///
    /// assert_eq!(rodeo.len(), 1);
    /// ```
    ///
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn len(&self) -> usize {
        self.strings.len()
    }

    /// Returns `true` if there are no currently interned strings
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::ThreadedRodeo;
    ///
    /// let rodeo = ThreadedRodeo::default();
    /// assert!(rodeo.is_empty());
    /// ```
    ///
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the number of strings that can be interned without a reallocation
    ///
    /// This is an unreliable measurement since the underlying hashmap is unreliable in its
    /// capacity measurement
    ///
    /// # Example
    ///
    /// ```no_run
    /// use lasso::{Spur, Capacity, ThreadedRodeo};
    ///
    /// let rodeo: ThreadedRodeo<Spur> = ThreadedRodeo::with_capacity(Capacity::for_strings(10));
    /// assert_eq!(rodeo.capacity(), 10);
    /// ```
    ///
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn capacity(&self) -> usize {
        self.strings.capacity()
    }

    /// Set the `ThreadedRodeo`'s maximum memory usage while in-flight
    ///
    /// Note that setting the maximum memory usage to below the currently allocated
    /// memory will do nothing
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn set_memory_limits(&self, memory_limits: MemoryLimits) {
        self.arena.lock().unwrap().max_memory_usage = memory_limits.max_memory_usage;
    }

    /// Get the `ThreadedRodeo`'s currently allocated memory
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn current_memory_usage(&self) -> usize {
        self.arena.lock().unwrap().memory_usage()
    }

    /// Get the `ThreadedRodeo`'s current maximum of allocated memory
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn max_memory_usage(&self) -> usize {
        self.arena.lock().unwrap().max_memory_usage
    }

    /// Consumes the current ThreadedRodeo, returning a [`RodeoReader`] to allow contention-free access of the interner
    /// from multiple threads
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::ThreadedRodeo;
    ///
    /// let rodeo = ThreadedRodeo::default();
    /// let key = rodeo.get_or_intern("Appear weak when you are strong, and strong when you are weak.");
    ///
    /// let rodeo_reader = rodeo.into_reader();
    /// assert_eq!(
    ///     "Appear weak when you are strong, and strong when you are weak.",
    ///     rodeo_reader.resolve(&key),
    /// );
    /// ```
    ///
    /// [`RodeoReader`]: crate::RodeoReader
    #[cfg_attr(feature = "inline-more", inline)]
    #[must_use]
    pub fn into_reader(self) -> RodeoReader<K, S> {
        // Take the strings vec from the old lasso
        let strings: Vec<&'static str> = {
            let mut strings = iter::from_fn(|| Some(None))
                .take(self.strings.len())
                .collect::<Vec<Option<&'static str>>>();

            for shard in self.strings.shards() {
                for (key, val) in shard.write().drain() {
                    unsafe {
                        // Safety: The keys of the dashmap are valid indices
                        *index_unchecked_mut!(strings, key.into_usize()) = Some(val.into_inner());
                    }
                }
            }

            strings.into_iter().map(|s| s.unwrap()).collect()
        };

        // Drain the DashMap by draining each of its buckets and creating a new hashmap to store their values
        let (map, hasher) = {
            let mut map: HashMap<K, (), ()> = HashMap::with_capacity_and_hasher(strings.len(), ());
            let hasher = self.map.hasher().clone();

            for shard in self.map.shards() {
                for (string, key) in shard.write().drain() {
                    let string: &str = string;

                    // Hash the string to use as the key's hash (See `Rodeo`'s documentation for details)
                    let hash = {
                        let mut state = hasher.build_hasher();
                        string.hash(&mut state);

                        state.finish()
                    };

                    // Get the entry of the hashmap and insert the key with our new, custom hash
                    let entry = map.raw_entry_mut().from_hash(hash, |key| {
                        // Safety: The index given by `key` will be in bounds of the strings vector
                        let key_string: &str =
                            unsafe { index_unchecked!(strings, key.into_usize()) };

                        // Compare the requested string against the key's string
                        string == key_string
                    });

                    match entry {
                        RawEntryMut::Occupied(_) => {
                            unreachable!("Keys in the hashmap are unique, so entries should never be occupied");
                        }

                        RawEntryMut::Vacant(entry) => {
                            // Insert the key with the hash of the string that it points to, reusing the hash we made earlier
                            entry.insert_with_hasher(hash, *key.get(), (), |key| {
                                let key_string: &str =
                                    unsafe { index_unchecked!(strings, key.into_usize()) };

                                let mut state = hasher.build_hasher();
                                key_string.hash(&mut state);

                                state.finish()
                            });
                        }
                    }
                }
            }

            (map, hasher)
        };

        // Safety: No other references outside of `map` and `strings` to the interned strings exist
        unsafe {
            RodeoReader::new(
                map,
                hasher,
                strings,
                mem::take(&mut *self.arena.lock().unwrap()),
            )
        }
    }

    /// Consumes the current ThreadedRodeo, returning a [`RodeoResolver`] to allow contention-free access of the interner
    /// from multiple threads with the lowest possible memory consumption
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::ThreadedRodeo;
    ///
    /// let rodeo = ThreadedRodeo::default();
    /// let key = rodeo.get_or_intern("Appear weak when you are strong, and strong when you are weak.");
    ///
    /// let rodeo_resolver = rodeo.into_resolver();
    /// assert_eq!(
    ///     "Appear weak when you are strong, and strong when you are weak.",
    ///     rodeo_resolver.resolve(&key),
    /// );
    /// ```
    ///
    /// [`RodeoResolver`]: crate::RodeoResolver
    #[cfg_attr(feature = "inline-more", inline)]
    #[must_use]
    pub fn into_resolver(self) -> RodeoResolver<K> {
        let mut strings = iter::from_fn(|| Some(None))
            .take(self.strings.len())
            .collect::<Vec<Option<&'static str>>>();

        for shard in self.strings.shards() {
            for (key, val) in shard.write().drain() {
                unsafe {
                    // Safety: The keys of the dashmap are valid indices
                    *index_unchecked_mut!(strings, key.into_usize()) = Some(val.into_inner());
                }
            }
        }

        // Safety: No other references to the strings exist
        unsafe {
            RodeoResolver::new(
                strings.into_iter().map(|s| s.unwrap()).collect(),
                mem::take(&mut *self.arena.lock().unwrap()),
            )
        }
    }
}

/// Creates a ThreadedRodeo using [`Spur`] as its key and [`RandomState`] as its hasher
///
/// [`Spur`]: crate::Spur
/// [`RandomState`]: index.html#cargo-features
impl Default for ThreadedRodeo<Spur, RandomState> {
    #[cfg_attr(feature = "inline-more", inline)]
    fn default() -> Self {
        Self::new()
    }
}

impl<K, S> Debug for ThreadedRodeo<K, S>
where
    K: Key + Hash + Debug,
    S: BuildHasher + Clone,
{
    #[cfg_attr(feature = "inline-more", inline)]
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_struct("Rodeo")
            .field("map", &self.map)
            .field("strings", &self.strings)
            .field("arena", &self.arena)
            .finish()
    }
}

unsafe impl<K: Sync, S: Sync> Sync for ThreadedRodeo<K, S> {}
unsafe impl<K: Send, S: Send> Send for ThreadedRodeo<K, S> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{hasher::RandomState, Capacity, MemoryLimits, MicroSpur};
    use core::num::NonZeroUsize;

    #[cfg(not(any(miri, feature = "no-std")))]
    use std::{sync::Arc, thread};

    compile! {
        if #[feature = "no-std"] {
            use alloc::string::ToString;
        } else {
            use std::string::ToString;
        }
    }

    #[test]
    fn new() {
        let _: ThreadedRodeo<Spur> = ThreadedRodeo::new();
    }

    #[test]
    fn with_capacity() {
        let rodeo: ThreadedRodeo<Spur> = ThreadedRodeo::with_capacity(Capacity::for_strings(10));
        // DashMap's capacity isn't reliable
        let _cap = rodeo.capacity();
    }

    #[test]
    fn with_hasher() {
        let rodeo: ThreadedRodeo<Spur, RandomState> =
            ThreadedRodeo::with_hasher(RandomState::new());

        let key = rodeo.get_or_intern("Test");
        assert_eq!("Test", rodeo.resolve(&key));
    }

    #[test]
    fn with_capacity_and_hasher() {
        let rodeo: ThreadedRodeo<Spur, RandomState> =
            ThreadedRodeo::with_capacity_and_hasher(Capacity::for_strings(10), RandomState::new());

        let key = rodeo.get_or_intern("Test");
        assert_eq!("Test", rodeo.resolve(&key));
    }

    #[test]
    fn get_or_intern() {
        let rodeo = ThreadedRodeo::default();

        let a = rodeo.get_or_intern("A");
        assert_eq!(a, rodeo.get_or_intern("A"));

        let b = rodeo.get_or_intern("B");
        assert_eq!(b, rodeo.get_or_intern("B"));

        let c = rodeo.get_or_intern("C");
        assert_eq!(c, rodeo.get_or_intern("C"));
    }

    #[test]
    #[cfg(not(any(miri, feature = "no-std")))]
    fn get_or_intern_threaded() {
        let rodeo = Arc::new(ThreadedRodeo::default());

        let moved = Arc::clone(&rodeo);
        thread::spawn(move || {
            let a = moved.get_or_intern("A");
            assert_eq!(a, moved.get_or_intern("A"));

            let b = moved.get_or_intern("B");
            assert_eq!(b, moved.get_or_intern("B"));

            let c = moved.get_or_intern("C");
            assert_eq!(c, moved.get_or_intern("C"));
        });

        let a = rodeo.get_or_intern("A");
        assert_eq!(a, rodeo.get_or_intern("A"));

        let b = rodeo.get_or_intern("B");
        assert_eq!(b, rodeo.get_or_intern("B"));

        let c = rodeo.get_or_intern("C");
        assert_eq!(c, rodeo.get_or_intern("C"));
    }

    #[test]
    fn try_get_or_intern() {
        let rodeo: ThreadedRodeo<MicroSpur> = ThreadedRodeo::new();

        for i in 0..u8::max_value() as usize - 1 {
            rodeo.get_or_intern(i.to_string());
        }

        let space = rodeo.try_get_or_intern("A").unwrap();
        assert_eq!("A", rodeo.resolve(&space));
        let space = rodeo.try_get_or_intern("A").unwrap();
        assert_eq!("A", rodeo.resolve(&space));

        assert!(rodeo.try_get_or_intern("C").is_none());
    }

    #[test]
    #[cfg(not(any(miri, feature = "no-std")))]
    fn try_get_or_intern_threaded() {
        let rodeo: Arc<ThreadedRodeo<MicroSpur>> = Arc::new(ThreadedRodeo::new());

        for i in 0..u8::max_value() as usize - 1 {
            rodeo.get_or_intern(i.to_string());
        }

        let moved = Arc::clone(&rodeo);
        thread::spawn(move || {
            let a = moved.try_get_or_intern("A");
            assert_eq!(a, moved.try_get_or_intern("A"));
            let a = moved.try_get_or_intern("A");
            assert_eq!(a, moved.try_get_or_intern("A"));

            let b = moved.try_get_or_intern("B");
            assert_eq!(b, moved.try_get_or_intern("B"));
            let b = moved.try_get_or_intern("B");
            assert_eq!(b, moved.try_get_or_intern("B"));

            let c = moved.try_get_or_intern("C");
            assert_eq!(c, moved.try_get_or_intern("C"));
            let c = moved.try_get_or_intern("C");
            assert_eq!(c, moved.try_get_or_intern("C"));
        });

        let a = rodeo.try_get_or_intern("A");
        assert_eq!(a, rodeo.try_get_or_intern("A"));
        let a = rodeo.try_get_or_intern("A");
        assert_eq!(a, rodeo.try_get_or_intern("A"));

        let b = rodeo.try_get_or_intern("B");
        assert_eq!(b, rodeo.try_get_or_intern("B"));
        let b = rodeo.try_get_or_intern("B");
        assert_eq!(b, rodeo.try_get_or_intern("B"));

        let c = rodeo.try_get_or_intern("C");
        assert_eq!(c, rodeo.try_get_or_intern("C"));
        let c = rodeo.try_get_or_intern("C");
        assert_eq!(c, rodeo.try_get_or_intern("C"));
    }

    #[test]
    fn get_or_intern_static() {
        let rodeo = ThreadedRodeo::default();

        let a = rodeo.get_or_intern_static("A");
        assert_eq!(a, rodeo.get_or_intern_static("A"));

        let b = rodeo.get_or_intern_static("B");
        assert_eq!(b, rodeo.get_or_intern_static("B"));

        let c = rodeo.get_or_intern_static("C");
        assert_eq!(c, rodeo.get_or_intern_static("C"));
    }

    #[test]
    #[cfg(not(any(miri, feature = "no-std")))]
    fn try_get_or_intern_static_threaded() {
        let rodeo: Arc<ThreadedRodeo<MicroSpur>> = Arc::new(ThreadedRodeo::new());

        let moved = Arc::clone(&rodeo);
        thread::spawn(move || {
            let a = moved.try_get_or_intern_static("A");
            assert_eq!(a, moved.try_get_or_intern("A"));
            let a = moved.try_get_or_intern_static("A");
            assert_eq!(a, moved.try_get_or_intern("A"));

            let b = moved.try_get_or_intern_static("B");
            assert_eq!(b, moved.try_get_or_intern("B"));
            let b = moved.try_get_or_intern_static("B");
            assert_eq!(b, moved.try_get_or_intern("B"));

            let c = moved.try_get_or_intern_static("C");
            assert_eq!(c, moved.try_get_or_intern("C"));
            let c = moved.try_get_or_intern_static("C");
            assert_eq!(c, moved.try_get_or_intern("C"));
        });

        let a = rodeo.try_get_or_intern_static("A");
        assert_eq!(a, rodeo.try_get_or_intern("A"));
        let a = rodeo.try_get_or_intern_static("A");
        assert_eq!(a, rodeo.try_get_or_intern("A"));

        let b = rodeo.try_get_or_intern_static("B");
        assert_eq!(b, rodeo.try_get_or_intern("B"));
        let b = rodeo.try_get_or_intern_static("B");
        assert_eq!(b, rodeo.try_get_or_intern("B"));

        let c = rodeo.try_get_or_intern_static("C");
        assert_eq!(c, rodeo.try_get_or_intern("C"));
        let c = rodeo.try_get_or_intern_static("C");
        assert_eq!(c, rodeo.try_get_or_intern("C"));
    }

    #[test]
    fn get() {
        let rodeo = ThreadedRodeo::default();
        let key = rodeo.get_or_intern("A");

        assert_eq!(Some(key), rodeo.get("A"));
    }

    #[test]
    #[cfg(not(any(miri, feature = "no-std")))]
    fn get_threaded() {
        let rodeo = Arc::new(ThreadedRodeo::default());
        let key = rodeo.get_or_intern("A");

        let moved = Arc::clone(&rodeo);
        thread::spawn(move || {
            assert_eq!(Some(key), moved.get("A"));
        });

        assert_eq!(Some(key), rodeo.get("A"));
    }

    #[test]
    fn resolve() {
        let rodeo = ThreadedRodeo::default();
        let key = rodeo.get_or_intern("A");

        assert_eq!("A", rodeo.resolve(&key));
    }

    #[test]
    #[should_panic]
    #[cfg(not(miri))]
    fn resolve_panics() {
        let rodeo = ThreadedRodeo::default();
        rodeo.resolve(&Spur::try_from_usize(100).unwrap());
    }

    #[test]
    #[cfg(not(any(miri, feature = "no-std")))]
    fn resolve_threaded() {
        let rodeo = Arc::new(ThreadedRodeo::default());
        let key = rodeo.get_or_intern("A");

        let moved = Arc::clone(&rodeo);
        thread::spawn(move || {
            assert_eq!("A", moved.resolve(&key));
        });

        assert_eq!("A", rodeo.resolve(&key));
    }

    #[test]
    #[cfg(not(any(miri, feature = "no-std", tarpaulin)))]
    fn resolve_panics_threaded() {
        let rodeo = Arc::new(ThreadedRodeo::default());
        let key = rodeo.get_or_intern("A");

        let moved = Arc::clone(&rodeo);
        let handle = thread::spawn(move || {
            assert_eq!("A", moved.resolve(&key));
            moved.resolve(&Spur::try_from_usize(100).unwrap());
        });

        assert_eq!("A", rodeo.resolve(&key));
        assert!(handle.join().is_err());
    }

    #[test]
    fn try_resolve() {
        let rodeo = ThreadedRodeo::default();
        let key = rodeo.get_or_intern("A");

        assert_eq!(Some("A"), rodeo.try_resolve(&key));
        assert_eq!(None, rodeo.try_resolve(&Spur::try_from_usize(100).unwrap()));
    }

    #[test]
    #[cfg(not(any(miri, feature = "no-std")))]
    fn try_resolve_threaded() {
        let rodeo = Arc::new(ThreadedRodeo::default());
        let key = rodeo.get_or_intern("A");

        let moved = Arc::clone(&rodeo);
        thread::spawn(move || {
            assert_eq!(Some("A"), moved.try_resolve(&key));
            assert_eq!(None, moved.try_resolve(&Spur::try_from_usize(100).unwrap()));
        });

        assert_eq!(Some("A"), rodeo.try_resolve(&key));
        assert_eq!(None, rodeo.try_resolve(&Spur::try_from_usize(100).unwrap()));
    }

    #[test]
    fn len() {
        let rodeo = ThreadedRodeo::default();
        rodeo.get_or_intern("A");
        rodeo.get_or_intern("B");
        rodeo.get_or_intern("C");

        assert_eq!(rodeo.len(), 3);
    }

    #[test]
    fn empty() {
        let rodeo = ThreadedRodeo::default();

        assert!(rodeo.is_empty());
    }

    // #[test]
    // fn clone() {
    //     let rodeo = ThreadedRodeo::default();
    //     let key = rodeo.get_or_intern("Test");
    //
    //     assert_eq!("Test", rodeo.resolve(&key));
    //
    //     let cloned = rodeo.clone();
    //     assert_eq!("Test", cloned.resolve(&key));
    //
    //     drop(rodeo);
    //
    //     assert_eq!("Test", cloned.resolve(&key));
    // }

    #[test]
    fn drops() {
        let _ = ThreadedRodeo::default();
    }

    #[test]
    #[cfg(not(any(miri, feature = "no-std")))]
    fn drop_threaded() {
        let rodeo = Arc::new(ThreadedRodeo::default());

        let moved = Arc::clone(&rodeo);
        thread::spawn(move || {
            let _ = moved;
        });
    }

    #[test]
    #[cfg(not(any(miri, feature = "no-std")))]
    fn debug() {
        let rodeo = ThreadedRodeo::default();
        println!("{:?}", rodeo);
    }

    #[test]
    fn into_resolver() {
        let rodeo = ThreadedRodeo::default();
        let key = rodeo.get_or_intern("A");

        let resolver = rodeo.into_resolver();
        assert_eq!("A", resolver.resolve(&key));
    }

    #[test]
    fn into_reader() {
        let rodeo = ThreadedRodeo::default();
        let key = rodeo.get_or_intern("A");

        let reader = rodeo.into_reader();
        assert_eq!("A", reader.resolve(&key));
    }

    #[test]
    fn memory_exhausted() {
        let rodeo: ThreadedRodeo<Spur> = ThreadedRodeo::with_capacity_and_memory_limits(
            Capacity::for_bytes(NonZeroUsize::new(10).unwrap()),
            MemoryLimits::for_memory_usage(10),
        );

        let string = rodeo.try_get_or_intern("0123456789").unwrap();
        assert_eq!(rodeo.resolve(&string), "0123456789");

        assert!(rodeo.try_get_or_intern("").is_none());
        assert!(rodeo.try_get_or_intern("").is_none());
        assert!(rodeo.try_get_or_intern("").is_none());

        assert_eq!(rodeo.resolve(&string), "0123456789");
    }

    #[test]
    #[cfg(not(any(miri, feature = "no-std")))]
    fn memory_exhausted_threaded() {
        let rodeo: Arc<ThreadedRodeo<Spur>> =
            Arc::new(ThreadedRodeo::with_capacity_and_memory_limits(
                Capacity::for_bytes(NonZeroUsize::new(10).unwrap()),
                MemoryLimits::for_memory_usage(10),
            ));

        let moved = Arc::clone(&rodeo);
        thread::spawn(move || {
            let string = moved.try_get_or_intern("0123456789").unwrap();
            assert_eq!(moved.resolve(&string), "0123456789");

            assert!(moved.try_get_or_intern("").is_none());
            assert!(moved.try_get_or_intern("").is_none());
            assert!(moved.try_get_or_intern("").is_none());

            assert_eq!(moved.resolve(&string), "0123456789");
        });

        let string = rodeo.try_get_or_intern("0123456789").unwrap();
        assert_eq!(rodeo.resolve(&string), "0123456789");

        assert!(rodeo.try_get_or_intern("").is_none());
        assert!(rodeo.try_get_or_intern("").is_none());
        assert!(rodeo.try_get_or_intern("").is_none());

        assert_eq!(rodeo.resolve(&string), "0123456789");
    }

    // TODO: Add a reason for should_panic once `Result`s are used
    #[test]
    #[should_panic]
    fn memory_exhausted_panics() {
        let rodeo: ThreadedRodeo<Spur> = ThreadedRodeo::with_capacity_and_memory_limits(
            Capacity::for_bytes(NonZeroUsize::new(10).unwrap()),
            MemoryLimits::for_memory_usage(10),
        );

        let string = rodeo.get_or_intern("0123456789");
        assert_eq!(rodeo.resolve(&string), "0123456789");

        rodeo.get_or_intern("");
    }

    #[test]
    fn with_capacity_memory_limits_and_hasher() {
        let rodeo: ThreadedRodeo<Spur, RandomState> =
            ThreadedRodeo::with_capacity_memory_limits_and_hasher(
                Capacity::default(),
                MemoryLimits::default(),
                RandomState::new(),
            );

        rodeo.get_or_intern("Test");
    }

    #[test]
    fn with_capacity_and_memory_limits() {
        let rodeo: ThreadedRodeo<Spur> = ThreadedRodeo::with_capacity_and_memory_limits(
            Capacity::default(),
            MemoryLimits::default(),
        );

        rodeo.get_or_intern("Test");
    }

    #[test]
    fn set_memory_limits() {
        let rodeo: ThreadedRodeo<Spur> = ThreadedRodeo::with_capacity_and_memory_limits(
            Capacity::for_bytes(NonZeroUsize::new(10).unwrap()),
            MemoryLimits::for_memory_usage(10),
        );

        let string1 = rodeo.try_get_or_intern("0123456789").unwrap();
        assert_eq!(rodeo.resolve(&string1), "0123456789");

        assert!(rodeo.try_get_or_intern("").is_none());
        assert!(rodeo.try_get_or_intern("").is_none());
        assert!(rodeo.try_get_or_intern("").is_none());

        assert_eq!(rodeo.resolve(&string1), "0123456789");

        rodeo.set_memory_limits(MemoryLimits::for_memory_usage(20));

        let string2 = rodeo.try_get_or_intern("9876543210").unwrap();
        assert_eq!(rodeo.resolve(&string2), "9876543210");

        assert!(rodeo.try_get_or_intern("").is_none());
        assert!(rodeo.try_get_or_intern("").is_none());
        assert!(rodeo.try_get_or_intern("").is_none());

        assert_eq!(rodeo.resolve(&string1), "0123456789");
        assert_eq!(rodeo.resolve(&string2), "9876543210");
    }

    #[test]
    fn memory_usage_stats() {
        let rodeo: ThreadedRodeo<Spur> = ThreadedRodeo::with_capacity_and_memory_limits(
            Capacity::for_bytes(NonZeroUsize::new(10).unwrap()),
            MemoryLimits::for_memory_usage(10),
        );

        rodeo.get_or_intern("0123456789");

        assert_eq!(rodeo.current_memory_usage(), 10);
        assert_eq!(rodeo.max_memory_usage(), 10);
    }
}
