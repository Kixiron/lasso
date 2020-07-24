use crate::{
    arena::Arena,
    hasher::RandomState,
    key::{Key, Spur},
    reader::RodeoReader,
    resolver::RodeoResolver,
    util::{Iter, Strings},
    Capacity,
};
use core::{
    fmt::{Debug, Formatter, Result as FmtResult},
    hash::{BuildHasher, Hash, Hasher},
};
use hashbrown::{hash_map::RawEntryMut, HashMap};

compile! {
    if #[feature = "no-std"] {
        use alloc::vec::Vec;
    }
}

/// A string interner that caches strings quickly with a minimal memory footprint,
/// returning a unique key to re-access it with `O(1)` times.
///
/// By default Rodeo uses the [`Spur`] type for keys and [`RandomState`] as its hasher
///
/// [`Spur`]: crate::Spur
/// [`RandomState`]: https://doc.rust-lang.org/std/collections/hash_map/struct.RandomState.html
pub struct Rodeo<K = Spur, S = RandomState> {
    /// Map that allows `str` -> `key` resolution
    ///
    /// This must be a `HashMap` (for now) since `raw_api`s are only avaliable for maps and not sets.
    /// The value of the map is `()` since the key is symbolically hashed as the string it represents and
    /// the hasher is also `()` so that we only store one hasher, the custom one contained in the `Rodeo` itself
    ///
    /// The keys stored in this map are not hashed as keys, they're inserted
    /// with the hashes of the strings that they point to
    ///
    /// For example, if the string `foo` has the key of `FooKey` and the hash of `0xF00`,
    /// then the hashmap will contain `FooKey` at the hashed location of `0xF00`.
    ///
    /// This allows us to only store references to the internally allocated strings once,
    /// which drastically decreases memory usage
    map: HashMap<K, (), ()>,
    /// The hasher of the map. This is stored outside of the map so that we can use
    /// custom hashing on the keys of the map without the map itself trying to do something else
    hasher: S,
    /// Vec that allows `key` -> `str` resolution
    pub(crate) strings: Vec<&'static str>,
    /// The arena that holds all allocated strings
    arena: Arena,
}

impl<K> Rodeo<K, RandomState>
where
    K: Key,
{
    /// Create a new Rodeo
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::{Rodeo, Spur};
    ///
    /// let mut rodeo: Rodeo<Spur> = Rodeo::new();
    /// let hello = rodeo.get_or_intern("Hello, ");
    /// let world = rodeo.get_or_intern("World!");
    ///
    /// assert_eq!("Hello, ", rodeo.resolve(&hello));
    /// assert_eq!("World!", rodeo.resolve(&world));
    /// ```
    ///
    #[inline]
    pub fn new() -> Self {
        Self::with_capacity_and_hasher(Capacity::default(), RandomState::new())
    }

    /// Create a new Rodeo with the specified capacity. The interner will be able to hold `capacity`
    /// strings without reallocating
    ///
    /// See [`Capacity`] for more information
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::{Rodeo, Capacity, Spur};
    ///
    /// let rodeo: Rodeo<Spur> = Rodeo::with_capacity(Capacity::for_strings(10));
    /// ```
    ///
    /// [`Capacity`]: crate::Capacity
    #[inline]
    pub fn with_capacity(capacity: Capacity) -> Self {
        Self::with_capacity_and_hasher(capacity, RandomState::new())
    }
}

impl<K, S> Rodeo<K, S>
where
    K: Key,
    S: BuildHasher,
{
    /// Creates an empty Rodeo which will use the given hasher for its internal hashmap
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::{Spur, Rodeo};
    /// use std::collections::hash_map::RandomState;
    ///
    /// let rodeo: Rodeo<Spur, RandomState> = Rodeo::with_hasher(RandomState::new());
    /// ```
    ///
    #[inline]
    pub fn with_hasher(hash_builder: S) -> Self {
        Self::with_capacity_and_hasher(Capacity::default(), hash_builder)
    }

    /// Creates a new Rodeo with the specified capacity that will use the given hasher for its internal hashmap
    ///
    /// See [`Capacity`] for more information
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::{Spur, Capacity, Rodeo};
    /// use std::collections::hash_map::RandomState;
    ///
    /// let rodeo: Rodeo<Spur, RandomState> = Rodeo::with_capacity_and_hasher(Capacity::for_strings(10), RandomState::new());
    /// ```
    ///
    /// [`Capacity`]: crate::Capacity
    #[inline]
    pub fn with_capacity_and_hasher(capacity: Capacity, hash_builder: S) -> Self {
        let Capacity { strings, bytes } = capacity;

        Self {
            map: HashMap::with_capacity_and_hasher(strings, ()),
            hasher: hash_builder,
            strings: Vec::with_capacity(strings),
            arena: Arena::with_capacity(bytes),
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
    /// use lasso::Rodeo;
    ///
    /// let mut rodeo = Rodeo::default();
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
    /// [`Spur`]: crate::Spur
    #[inline]
    pub fn get_or_intern<T>(&mut self, val: T) -> K
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
    /// use lasso::Rodeo;
    ///
    /// let mut rodeo = Rodeo::default();
    ///
    /// // Interned the string
    /// let key = rodeo.try_get_or_intern("Strings of things with wings and dings").unwrap();
    /// assert_eq!("Strings of things with wings and dings", rodeo.resolve(&key));
    ///
    /// // No string was interned, as it was already contained
    /// let key = rodeo.try_get_or_intern("Strings of things with wings and dings").unwrap();
    /// assert_eq!("Strings of things with wings and dings", rodeo.resolve(&key));
    /// ```
    ///
    #[inline]
    pub fn try_get_or_intern<T>(&mut self, val: T) -> Option<K>
    where
        T: AsRef<str>,
    {
        let Self {
            map,
            hasher,
            strings,
            arena,
        } = self;

        let string_slice: &str = val.as_ref();

        // Make a hash of the requested string
        let hash = {
            let mut state = hasher.build_hasher();
            string_slice.hash(&mut state);

            state.finish()
        };

        // Get the map's entry that the string should occupy
        let entry = map.raw_entry_mut().from_hash(hash, |key| {
            // Safety: The index given by `key` will be in bounds of the strings vector
            let key_string: &str = unsafe { index_unchecked!(strings, key.into_usize()) };

            // Compare the requested string against the key's string
            string_slice == key_string
        });

        let key = match entry {
            // The string already exists, so return its key
            RawEntryMut::Occupied(entry) => *entry.into_key(),

            // The string does not yet exist, so insert it and create its key
            RawEntryMut::Vacant(entry) => {
                // Create the key from the vec's index that the string will hold
                let key = K::try_from_usize(strings.len())?;

                // Allocate the string in the arena
                // Safety: The returned strings will be dropped before the arena that created them is
                let allocated = unsafe { arena.store_str(string_slice) };

                // Push the allocated string to the strings vector
                strings.push(allocated);

                // Insert the key with the hash of the string that it points to, reusing the hash we made earlier
                entry.insert_with_hasher(hash, key, (), |key| {
                    let key_string: &str = unsafe { index_unchecked!(strings, key.into_usize()) };

                    let mut state = hasher.build_hasher();
                    key_string.hash(&mut state);

                    state.finish()
                });

                key
            }
        };

        Some(key)
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
    /// use lasso::Rodeo;
    ///
    /// let mut rodeo = Rodeo::default();
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
    #[inline]
    pub fn get_or_intern_static(&mut self, string: &'static str) -> K {
        self.try_get_or_intern_static(string)
            .expect("Failed to get or intern static string")
    }

    /// Get the key for a static string, interning it if it does not yet exist
    ///
    /// This will not reallocate or copy the given string but will instead just store it
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::Rodeo;
    ///
    /// let mut rodeo = Rodeo::default();
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
    #[inline]
    pub fn try_get_or_intern_static(&mut self, string: &'static str) -> Option<K> {
        let Self {
            map,
            hasher,
            strings,
            ..
        } = self;

        // Make a hash of the requested string
        let hash = {
            let mut state = hasher.build_hasher();
            string.hash(&mut state);

            state.finish()
        };

        // Get the map's entry that the string should occupy
        let entry = map.raw_entry_mut().from_hash(hash, |key| {
            // Safety: The index given by `key` will be in bounds of the strings vector
            let key_string: &str = unsafe { index_unchecked!(strings, key.into_usize()) };

            // Compare the requested string against the key's string
            string == key_string
        });

        let key = match entry {
            // The string already exists, so return its key
            RawEntryMut::Occupied(entry) => *entry.into_key(),

            // The string does not yet exist, so insert it and create its key
            RawEntryMut::Vacant(entry) => {
                // Create the key from the vec's index that the string will hold
                let key = K::try_from_usize(strings.len())?;

                // Push the static string to the strings vector
                strings.push(string);

                // Insert the key with the hash of the string that it points to, reusing the hash we made earlier
                entry.insert_with_hasher(hash, key, (), |key| {
                    let key_string: &str = unsafe { index_unchecked!(strings, key.into_usize()) };

                    let mut state = hasher.build_hasher();
                    key_string.hash(&mut state);

                    state.finish()
                });

                key
            }
        };

        Some(key)
    }

    /// Get the key value of a string, returning `None` if it doesn't exist
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::Rodeo;
    ///
    /// let mut rodeo = Rodeo::default();
    ///
    /// let key = rodeo.get_or_intern("Strings of things with wings and dings");
    /// assert_eq!(Some(key), rodeo.get("Strings of things with wings and dings"));
    ///
    /// assert_eq!(None, rodeo.get("This string isn't interned"));
    /// ```
    ///
    #[inline]
    pub fn get<T>(&self, val: T) -> Option<K>
    where
        T: AsRef<str>,
    {
        let string_slice: &str = val.as_ref();

        // Make a hash of the requested string
        let hash = {
            let mut state = self.hasher.build_hasher();
            string_slice.hash(&mut state);

            state.finish()
        };

        // Get the map's entry that the string should occupy
        let entry = self.map.raw_entry().from_hash(hash, |key| {
            // Safety: The index given by `key` will be in bounds of the strings vector
            let key_string: &str = unsafe { index_unchecked!(self.strings, key.into_usize()) };

            // Compare the requested string against the
            string_slice == key_string
        });

        entry.map(|(key, ())| *key)
    }

    /// Resolves a string by its key. Only keys made by the current Rodeo may be used
    ///
    /// # Panics
    ///
    /// Panics if the key is out of bounds
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::Rodeo;
    ///
    /// let mut rodeo = Rodeo::default();
    ///
    /// let key = rodeo.get_or_intern("Strings of things with wings and dings");
    /// assert_eq!("Strings of things with wings and dings", rodeo.resolve(&key));
    /// ```
    ///
    #[inline]
    pub fn resolve<'a>(&'a self, key: &K) -> &'a str {
        // Safety: The call to get_unchecked's safety relies on the Key::into_usize impl
        // being symmetric and the caller having not fabricated a key. If the impl is sound
        // and symmetric, then it will succeed, as the usize used to create it is a valid
        // index into self.strings
        unsafe {
            assert!(key.into_usize() < self.strings.len());
            self.strings.get_unchecked(key.into_usize())
        }
    }

    /// Resolves a string by its key, returning `None` if it's out of bounds. Only keys made by the
    /// current Rodeo may be used
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::Rodeo;
    ///
    /// let mut rodeo = Rodeo::default();
    ///
    /// let key = rodeo.get_or_intern("Strings of things with wings and dings");
    /// assert_eq!(Some("Strings of things with wings and dings"), rodeo.try_resolve(&key));
    /// ```
    ///
    #[inline]
    pub fn try_resolve<'a>(&'a self, key: &K) -> Option<&'a str> {
        // Safety: The call to get_unchecked's safety relies on the Key::into_usize impl
        // being symmetric and the caller having not fabricated a key. If the impl is sound
        // and symmetric, then it will succeed, as the usize used to create it is a valid
        // index into self.strings
        unsafe {
            if key.into_usize() < self.strings.len() {
                Some(self.strings.get_unchecked(key.into_usize()))
            } else {
                None
            }
        }
    }

    /// Resolves a string by its key, without bounds checks
    ///
    /// # Safety
    ///
    /// The key must be valid for the current interner
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::Rodeo;
    ///
    /// let mut rodeo = Rodeo::default();
    ///
    /// let key = rodeo.get_or_intern("Strings of things with wings and dings");
    /// unsafe {
    ///     assert_eq!("Strings of things with wings and dings", rodeo.resolve_unchecked(&key));
    /// }
    /// ```
    ///
    #[inline]
    pub unsafe fn resolve_unchecked<'a>(&'a self, key: &K) -> &'a str {
        self.strings.get_unchecked(key.into_usize())
    }

    /// Gets the number of interned strings
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::Rodeo;
    ///
    /// let mut rodeo = Rodeo::default();
    /// rodeo.get_or_intern("Documentation often has little hidden bits in it");
    ///
    /// assert_eq!(rodeo.len(), 1);
    /// ```
    ///
    #[inline]
    pub fn len(&self) -> usize {
        self.strings.len()
    }

    /// Returns `true` if there are no currently interned strings
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::Rodeo;
    ///
    /// let rodeo = Rodeo::default();
    /// assert!(rodeo.is_empty());
    /// ```
    ///
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the number of strings that can be interned without a reallocation
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::{Spur, Capacity, Rodeo};
    ///
    /// let rodeo: Rodeo<Spur> = Rodeo::with_capacity(Capacity::for_strings(10));
    /// assert_eq!(rodeo.capacity(), 10);
    /// ```
    ///
    #[inline]
    pub fn capacity(&self) -> usize {
        self.strings.capacity()
    }

    // TODO: Examples here

    /// Returns an iterator over the interned strings and their key values
    #[inline]
    pub fn iter(&self) -> Iter<'_, K> {
        Iter::from_rodeo(self)
    }

    /// Returns an iterator over the interned strings
    #[inline]
    pub fn strings(&self) -> Strings<'_, K> {
        Strings::from_rodeo(self)
    }
}

impl<K, S> Rodeo<K, S>
where
    K: Key + Default,
    S: BuildHasher + Clone,
{
    /// Consumes the current Rodeo, returning a [`RodeoReader`] to allow contention-free access of the interner
    /// from multiple threads
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::Rodeo;
    ///
    /// let mut rodeo = Rodeo::default();
    /// let key = rodeo.get_or_intern("Appear weak when you are strong, and strong when you are weak.");
    ///
    /// let read_only_rodeo = rodeo.into_reader();
    /// assert_eq!(
    ///     "Appear weak when you are strong, and strong when you are weak.",
    ///     read_only_rodeo.resolve(&key),
    /// );
    /// ```
    ///
    /// [`RodeoReader`]: crate::RodeoReader
    #[inline]
    #[must_use]
    pub fn into_reader(self) -> RodeoReader<K, S> {
        let Self {
            map,
            hasher,
            strings,
            arena,
        } = self;

        // Safety: No other references outside of `map` and `strings` to the interned strings exist
        unsafe { RodeoReader::new(map, hasher, strings, arena) }
    }

    /// Consumes the current Rodeo, returning a [`RodeoResolver`] to allow contention-free access of the interner
    /// from multiple threads with the lowest possible memory consumption
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::Rodeo;
    ///
    /// let mut rodeo = Rodeo::default();
    /// let key = rodeo.get_or_intern("Appear weak when you are strong, and strong when you are weak.");
    ///
    /// let resolver_rodeo = rodeo.into_resolver();
    /// assert_eq!(
    ///     "Appear weak when you are strong, and strong when you are weak.",
    ///     resolver_rodeo.resolve(&key),
    /// );
    /// ```
    ///
    /// [`RodeoResolver`]: crate::RodeoResolver
    #[inline]
    #[must_use]
    pub fn into_resolver(self) -> RodeoResolver<K> {
        let Rodeo { strings, arena, .. } = self;

        // Safety: No other references to the strings exist
        unsafe { RodeoResolver::new(strings, arena) }
    }
}

/// Creates a Rodeo using [`Spur`] as its key and [`RandomState`] as its hasher
///
/// [`Spur`]: crate::Spur
/// [`RandomState`]: index.html#cargo-features
impl Default for Rodeo<Spur, RandomState> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Debug, S> Debug for Rodeo<K, S> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_struct("Rodeo")
            .field("map", &self.map)
            .field("strings", &self.strings)
            .finish()
    }
}

unsafe impl<K: Send, S: Send> Send for Rodeo<K, S> {}

#[cfg(test)]
mod tests {
    use crate::{hasher::RandomState, Capacity, Key, MicroSpur, Rodeo, Spur};

    compile! {
        if #[feature = "no-std"] {
            use alloc::string::ToString;
        }
    }

    #[test]
    fn new() {
        let mut rodeo: Rodeo<Spur> = Rodeo::new();
        rodeo.get_or_intern("Test");
    }

    #[test]
    fn with_capacity() {
        let mut rodeo: Rodeo<Spur> = Rodeo::with_capacity(Capacity::for_strings(10));
        assert_eq!(rodeo.capacity(), 10);

        rodeo.get_or_intern("Test");
        rodeo.get_or_intern("Test1");
        rodeo.get_or_intern("Test2");
        rodeo.get_or_intern("Test3");
        rodeo.get_or_intern("Test4");
        rodeo.get_or_intern("Test5");
        rodeo.get_or_intern("Test6");
        rodeo.get_or_intern("Test7");
        rodeo.get_or_intern("Test8");
        rodeo.get_or_intern("Test9");

        assert_eq!(rodeo.len(), rodeo.capacity());
    }

    #[test]
    fn with_hasher() {
        let mut rodeo: Rodeo<Spur, RandomState> = Rodeo::with_hasher(RandomState::new());
        let key = rodeo.get_or_intern("Test");
        assert_eq!("Test", rodeo.resolve(&key));

        let mut rodeo: Rodeo<Spur, ahash::RandomState> =
            Rodeo::with_hasher(ahash::RandomState::new());
        let key = rodeo.get_or_intern("Test");
        assert_eq!("Test", rodeo.resolve(&key));
    }

    #[test]
    fn with_capacity_and_hasher() {
        let mut rodeo: Rodeo<Spur, RandomState> =
            Rodeo::with_capacity_and_hasher(Capacity::for_strings(10), RandomState::new());
        assert_eq!(rodeo.capacity(), 10);

        rodeo.get_or_intern("Test");
        rodeo.get_or_intern("Test1");
        rodeo.get_or_intern("Test2");
        rodeo.get_or_intern("Test3");
        rodeo.get_or_intern("Test4");
        rodeo.get_or_intern("Test5");
        rodeo.get_or_intern("Test6");
        rodeo.get_or_intern("Test7");
        rodeo.get_or_intern("Test8");
        rodeo.get_or_intern("Test9");

        assert_eq!(rodeo.len(), rodeo.capacity());

        let mut rodeo: Rodeo<Spur, ahash::RandomState> =
            Rodeo::with_capacity_and_hasher(Capacity::for_strings(10), ahash::RandomState::new());
        assert_eq!(rodeo.capacity(), 10);

        rodeo.get_or_intern("Test");
        rodeo.get_or_intern("Test1");
        rodeo.get_or_intern("Test2");
        rodeo.get_or_intern("Test3");
        rodeo.get_or_intern("Test4");
        rodeo.get_or_intern("Test5");
        rodeo.get_or_intern("Test6");
        rodeo.get_or_intern("Test7");
        rodeo.get_or_intern("Test8");
        rodeo.get_or_intern("Test9");

        assert_eq!(rodeo.len(), rodeo.capacity());
    }

    #[test]
    fn get_or_intern() {
        let mut rodeo = Rodeo::default();
        let a = rodeo.get_or_intern("A");
        assert_eq!(a, rodeo.get_or_intern("A"));

        let b = rodeo.get_or_intern("B");
        assert_eq!(b, rodeo.get_or_intern("B"));

        let c = rodeo.get_or_intern("C");
        assert_eq!(c, rodeo.get_or_intern("C"));
    }

    #[test]
    fn try_get_or_intern() {
        let mut rodeo: Rodeo<MicroSpur> = Rodeo::new();

        for i in 0..u8::max_value() as usize - 1 {
            rodeo.get_or_intern(i.to_string());
        }

        let space = rodeo.try_get_or_intern("A").unwrap();
        assert_eq!(Some(space), rodeo.try_get_or_intern("A"));
        assert_eq!("A", rodeo.resolve(&space));

        assert!(rodeo.try_get_or_intern("C").is_none());
    }

    #[test]
    fn get_or_intern_static() {
        let mut rodeo = Rodeo::default();
        let a = rodeo.get_or_intern_static("A");
        assert_eq!(a, rodeo.get_or_intern_static("A"));

        let b = rodeo.get_or_intern_static("B");
        assert_eq!(b, rodeo.get_or_intern_static("B"));

        let c = rodeo.get_or_intern_static("C");
        assert_eq!(c, rodeo.get_or_intern_static("C"));
    }

    #[test]
    fn try_get_or_intern_static() {
        use core::pin::Pin;
        compile! {
            if #[feature = "no-std"] {
                use alloc::vec::Vec;
            }
        }

        let mut strings = Vec::new();
        let mut rodeo: Rodeo<MicroSpur> = Rodeo::new();

        for i in 0..u8::max_value() as usize - 1 {
            let string = Pin::new(i.to_string().into_boxed_str());
            let static_ref = unsafe { core::mem::transmute(Pin::into_inner(string.as_ref())) };
            strings.push(string);

            rodeo.get_or_intern_static(static_ref);
        }

        let space = rodeo.try_get_or_intern_static("A").unwrap();
        assert_eq!(Some(space), rodeo.try_get_or_intern_static("A"));
        assert_eq!("A", rodeo.resolve(&space));

        assert!(rodeo.try_get_or_intern_static("C").is_none());
    }

    #[test]
    fn get() {
        let mut rodeo = Rodeo::default();
        let key = rodeo.get_or_intern("A");

        assert_eq!(Some(key), rodeo.get("A"));
    }

    #[test]
    fn resolve() {
        let mut rodeo = Rodeo::default();
        let key = rodeo.get_or_intern("A");

        assert_eq!("A", rodeo.resolve(&key));
    }

    #[test]
    #[should_panic]
    #[cfg(not(miri))]
    fn resolve_panics() {
        let rodeo = Rodeo::default();
        rodeo.resolve(&Spur::try_from_usize(100).unwrap());
    }

    #[test]
    fn try_resolve() {
        let mut rodeo = Rodeo::default();
        let key = rodeo.get_or_intern("A");

        assert_eq!(Some("A"), rodeo.try_resolve(&key));
        assert_eq!(None, rodeo.try_resolve(&Spur::try_from_usize(100).unwrap()));
    }

    #[test]
    fn resolve_unchecked() {
        let mut rodeo = Rodeo::default();
        let key = rodeo.get_or_intern("A");

        unsafe {
            assert_eq!("A", rodeo.resolve_unchecked(&key));
        }
    }

    #[test]
    fn len() {
        let mut rodeo = Rodeo::default();
        rodeo.get_or_intern("A");
        rodeo.get_or_intern("B");
        rodeo.get_or_intern("C");

        assert_eq!(rodeo.len(), 3);
    }

    #[test]
    fn empty() {
        let rodeo = Rodeo::default();

        assert!(rodeo.is_empty());
    }

    // #[test]
    // fn clone_rodeo() {
    //     let mut rodeo = Rodeo::default();
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
    fn drop_rodeo() {
        let _ = Rodeo::default();
    }

    #[test]
    fn iter() {
        let mut rodeo = Rodeo::default();
        let a = rodeo.get_or_intern("a");
        let b = rodeo.get_or_intern("b");
        let c = rodeo.get_or_intern("c");

        let mut rodeo = rodeo.iter();
        assert_eq!(Some((a, "a")), rodeo.next());
        assert_eq!(Some((b, "b")), rodeo.next());
        assert_eq!(Some((c, "c")), rodeo.next());
        assert_eq!(None, rodeo.next());
    }

    #[test]
    fn strings() {
        let mut rodeo = Rodeo::default();
        rodeo.get_or_intern("a");
        rodeo.get_or_intern("b");
        rodeo.get_or_intern("c");

        let mut rodeo = rodeo.strings();
        assert_eq!(Some("a"), rodeo.next());
        assert_eq!(Some("b"), rodeo.next());
        assert_eq!(Some("c"), rodeo.next());
        assert_eq!(None, rodeo.next());
    }

    #[test]
    #[cfg(not(any(feature = "no-std", feature = "ahasher")))]
    fn debug() {
        let rodeo = Rodeo::default();
        println!("{:?}", rodeo);
    }

    // Regression test for https://github.com/Kixiron/lasso/issues/7
    #[test]
    fn wrong_keys() {
        let mut rodeo = Rodeo::default();

        rodeo.get_or_intern("a");
        rodeo.get_or_intern("b");
        rodeo.get_or_intern("c");
        rodeo.get_or_intern("d");
        rodeo.get_or_intern("e");
        rodeo.get_or_intern("f");
        rodeo.get_or_intern("g");
        rodeo.get_or_intern("h");
        rodeo.get_or_intern("i");
        rodeo.get_or_intern("j");
        rodeo.get_or_intern("k");
        rodeo.get_or_intern("l");
        rodeo.get_or_intern("m");
        rodeo.get_or_intern("n");
        rodeo.get_or_intern("o");
        rodeo.get_or_intern("p");
        rodeo.get_or_intern("q");
        rodeo.get_or_intern("r");
        rodeo.get_or_intern("s");
        rodeo.get_or_intern("t");
        rodeo.get_or_intern("u");
        rodeo.get_or_intern("v");
        rodeo.get_or_intern("w");
        rodeo.get_or_intern("x");
        rodeo.get_or_intern("y");
        rodeo.get_or_intern("z");
        rodeo.get_or_intern("aa");
        rodeo.get_or_intern("bb");
        rodeo.get_or_intern("cc");
        rodeo.get_or_intern("dd");
        rodeo.get_or_intern("ee");
        rodeo.get_or_intern("ff");
        rodeo.get_or_intern("gg");
        rodeo.get_or_intern("hh");
        rodeo.get_or_intern("ii");
        rodeo.get_or_intern("jj");
        rodeo.get_or_intern("kk");
        rodeo.get_or_intern("ll");
        rodeo.get_or_intern("mm");
        rodeo.get_or_intern("nn");
        rodeo.get_or_intern("oo");
        rodeo.get_or_intern("pp");
        rodeo.get_or_intern("qq");
        rodeo.get_or_intern("rr");
        rodeo.get_or_intern("ss");
        rodeo.get_or_intern("tt");
        rodeo.get_or_intern("uu");
        rodeo.get_or_intern("vv");
        rodeo.get_or_intern("ww");
        rodeo.get_or_intern("xx");
        rodeo.get_or_intern("yy");
        rodeo.get_or_intern("zz");
        rodeo.get_or_intern("aaa");
        rodeo.get_or_intern("bbb");
        rodeo.get_or_intern("ccc");

        let var = rodeo.get_or_intern("ddd");

        rodeo.get_or_intern("eee");

        let var2 = rodeo.get_or_intern("ddd");
        assert_eq!(var, var2);
    }
}
