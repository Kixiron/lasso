use crate::{
    arena::Arena,
    hasher::{HashMap, RandomState},
    internable::Internable,
    key::{Key, Spur},
    reader::RodeoReader,
    resolver::RodeoResolver,
    util::{Iter, Strings},
};

use core::hash::BuildHasher;

compile! {
    if #[feature = "no-std"] {
        use alloc::vec::Vec;
    }
}

/// A string interner that caches strings quickly with a minimal memory footprint,
/// returning a unique key to re-access it with `O(1)` internment and resolution.
///
/// By default Rodeo uses the [`Spur`] type for keys and [`RandomState`] as its hasher
///
/// [`RandomState`]: index.html#cargo-features
#[derive(Debug)]
pub struct Rodeo<V = str, K = Spur, S = RandomState>
where
    V: Internable + ?Sized,
    K: Key,
    S: BuildHasher + Clone,
{
    /// Map that allows `str` -> `key` resolution
    map: HashMap<&'static V, K, S>,
    /// Vec that allows `key` -> `str` resolution
    pub(crate) strings: Vec<&'static V>,
    /// The arena that holds all allocated strings
    arena: Arena<V::Raw>,
}

impl<V, K> Rodeo<V, K, RandomState>
where
    V: Internable + ?Sized,
    K: Key,
{
    /// Create a new Rodeo
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::{Rodeo, Spur};
    ///
    /// let mut rodeo: Rodeo<str, Spur> = Rodeo::new();
    /// let hello = rodeo.get_or_intern("Hello, ");
    /// let world = rodeo.get_or_intern("World!");
    ///
    /// assert_eq!("Hello, ", rodeo.resolve(&hello));
    /// assert_eq!("World!", rodeo.resolve(&world));
    /// ```
    ///
    #[inline]
    pub fn new() -> Self {
        Self {
            map: HashMap::with_hasher(RandomState::new()),
            strings: Vec::new(),
            arena: Arena::default(),
        }
    }

    /// Create a new Rodeo with the specified capacity. The interner will be able to hold `capacity`
    /// strings without reallocating. If capacity is 0, the interner will not allocate.
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::{Rodeo, Spur};
    ///
    /// let rodeo: Rodeo<str, Spur> = Rodeo::with_capacity(10);
    /// ```
    ///
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            map: HashMap::with_capacity_and_hasher(capacity, RandomState::new()),
            strings: Vec::with_capacity(capacity),
            arena: Arena::default(),
        }
    }
}

impl<V, K, S> Rodeo<V, K, S>
where
    V: Internable + ?Sized,
    K: Key,
    S: BuildHasher + Clone,
{
    /// Creates an empty Rodeo which will use the given hasher for its internal hashmap
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::{Spur, Rodeo};
    /// use std::collections::hash_map::RandomState;
    ///
    /// let rodeo: Rodeo<str, Spur, RandomState> = Rodeo::with_hasher(RandomState::new());
    /// ```
    ///
    #[inline]
    pub fn with_hasher(hash_builder: S) -> Self {
        Self {
            map: HashMap::with_hasher(hash_builder),
            strings: Vec::new(),
            arena: Arena::default(),
        }
    }

    /// Creates a new Rodeo with the specified capacity that will use the given hasher for its internal hashmap
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::{Spur, Rodeo};
    /// use std::collections::hash_map::RandomState;
    ///
    /// let rodeo: Rodeo<str, Spur, RandomState> = Rodeo::with_capacity_and_hasher(10, RandomState::new());
    /// ```
    ///
    #[inline]
    pub fn with_capacity_and_hasher(capacity: usize, hash_builder: S) -> Self {
        Self {
            map: HashMap::with_capacity_and_hasher(capacity, hash_builder),
            strings: Vec::with_capacity(capacity),
            arena: Arena::default(),
        }
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
    /// let key = rodeo.get_or_intern("Strings of things with wings and dings");
    /// assert_eq!("Strings of things with wings and dings", rodeo.resolve(&key));
    ///
    /// // No string was interned, as it was already contained
    /// let key = rodeo.get_or_intern("Strings of things with wings and dings");
    /// assert_eq!("Strings of things with wings and dings", rodeo.resolve(&key));
    /// ```
    ///
    #[inline]
    pub fn get_or_intern<T>(&mut self, val: T) -> K
    where
        T: AsRef<V>,
    {
        // When the feature-set supports it, only hash the value once
        compile_expr! {
            if #[any(feature = "nightly", feature = "hashbrown-table")] {
                use core::hash::Hasher;

                compile! {
                    if #[feature = "hashbrown-table"] {
                        use hashbrown::hash_map::RawEntryMut;
                    } else if #[feature = "nightly"] {
                        use std::collections::hash_map::RawEntryMut;
                    }
                }

                let mut hasher = self.map.hasher().build_hasher();
                val.as_ref().hash(&mut hasher);
                let hash = hasher.finish();

                match self.map.raw_entry_mut().from_key_hashed_nocheck(hash, val.as_ref()) {
                    RawEntryMut::Occupied(entry) => *entry.get(),
                    RawEntryMut::Vacant(entry) => {
                        let key = K::try_from_usize(self.strings.len()).expect("Failed to get or intern string");

                        // Safety: The drop impl removes all references before the arena is dropped
                        let item = unsafe { V::from_raw(self.arena.store_slice(val.as_ref().to_raw())) };

                        entry.insert_hashed_nocheck(hash, item, key);
                        self.strings.push(item);

                        key
                    }
                }
            } else {
                if let Some(key) = self.get(&val) {
                    key
                } else {
                    let key = K::try_from_usize(self.strings.len()).expect("Failed to get or intern string");

                    // Safety: The drop impl removes all references before the arena is dropped
                    let item = unsafe { V::from_raw(self.arena.store_slice(val.as_ref().to_raw())) };

                    self.map.insert(item, key);
                    self.strings.push(item);

                    key
                }
            }
        }
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
    /// let key = rodeo.get_or_intern("Strings of things with wings and dings");
    /// assert_eq!("Strings of things with wings and dings", rodeo.resolve(&key));
    ///
    /// // No string was interned, as it was already contained
    /// let key = rodeo.get_or_intern("Strings of things with wings and dings");
    /// assert_eq!("Strings of things with wings and dings", rodeo.resolve(&key));
    /// ```
    ///
    #[inline]
    pub fn try_get_or_intern<T>(&mut self, val: T) -> Option<K>
    where
        T: AsRef<V>,
    {
        // When the feature-set supports it, only hash the value once
        compile_expr! {
            if #[any(feature = "nightly", feature = "hashbrown-table")] {
                use core::hash::Hasher;

                compile! {
                    if #[feature = "hashbrown-table"] {
                        use hashbrown::hash_map::RawEntryMut;
                    } else if #[feature = "nightly"] {
                        use std::collections::hash_map::RawEntryMut;
                    }
                }

                let mut hasher = self.map.hasher().build_hasher();
                val.as_ref().hash(&mut hasher);
                let hash = hasher.finish();

                match self.map.raw_entry_mut().from_key_hashed_nocheck(hash, val.as_ref()) {
                    RawEntryMut::Occupied(entry) => Some(*entry.get()),
                    RawEntryMut::Vacant(entry) => {
                        let key = K::try_from_usize(self.strings.len())?;

                        // Safety: The drop impl removes all references before the arena is dropped
                        let item = unsafe { V::from_raw(self.arena.store_slice(val.as_ref().to_raw())) };

                        entry.insert_hashed_nocheck(hash, item, key);
                        self.strings.push(item);

                        Some(key)
                    }
                }
            } else {
                if let Some(key) = self.get(&val) {
                    Some(key)
                } else {
                    let key = K::try_from_usize(self.strings.len())?;

                    // Safety: The drop impl removes all references before the arena is dropped
                    let item = unsafe { V::from_raw(self.arena.store_slice(val.as_ref().to_raw())) };

                    self.map.insert(item, key);
                    self.strings.push(item);

                    Some(key)
                }
            }
        }
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
        T: AsRef<V>,
    {
        self.map.get(val.as_ref()).map(|&k| k)
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
    pub fn resolve<'a>(&'a self, key: &K) -> &'a V {
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
    pub fn try_resolve<'a>(&'a self, key: &K) -> Option<&'a V> {
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
    pub unsafe fn resolve_unchecked<'a>(&'a self, key: &K) -> &'a V {
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
    /// use lasso::{Spur, Rodeo};
    ///
    /// let rodeo: Rodeo<str, Spur> = Rodeo::with_capacity(10);
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
    pub fn iter(&self) -> Iter<'_, V, K> {
        Iter::from_rodeo(self)
    }

    /// Returns an iterator over the interned strings
    #[inline]
    pub fn strings(&self) -> Strings<'_, V, K> {
        Strings::from_rodeo(self)
    }
}

impl<V, K, S> Rodeo<V, K, S>
where
    V: Internable + ?Sized,
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
    pub fn into_reader(self) -> RodeoReader<V, K, S> {
        let Self {
            map,
            strings,
            arena,
        } = self;

        // Safety: No other references outside of `map` and `strings` to the interned strings exist
        unsafe {
            RodeoReader::new(
                map,
                strings,
                arena,
            )
        }
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
    pub fn into_resolver(self) -> RodeoResolver<V, K> {
        let Rodeo {
            map: _map,
            strings,
            arena,
        } = self;

        // Safety: No other references to the strings exist
        unsafe { RodeoResolver::new(strings, arena) }
    }
}

/// Creates a Rodeo using [`Spur`] as its key and [`RandomState`] as its hasher
///
/// [`Spur`]: crate::Spur
/// [`RandomState`]: index.html#cargo-features
impl Default for Rodeo<str, Spur, RandomState> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl<V, K, S> Send for Rodeo<V, K, S>
where
    V: Internable + ?Sized + Send,
    K: Key + Send,
    S: BuildHasher + Clone + Send,
{
}

#[cfg(test)]
mod tests {
    use crate::{hasher::RandomState, Key, MicroSpur, Rodeo, Spur};

    compile! {
        if #[feature = "no-std"] {
            use alloc::string::ToString;
        }
    }

    #[test]
    fn new() {
        let mut rodeo: Rodeo<str, Spur> = Rodeo::new();
        rodeo.get_or_intern("Test");
    }

    #[test]
    fn with_capacity() {
        let mut rodeo: Rodeo<str, Spur> = Rodeo::with_capacity(10);
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
        let mut rodeo: Rodeo<str, Spur, RandomState> = Rodeo::with_hasher(RandomState::new());
        let key = rodeo.get_or_intern("Test");
        assert_eq!("Test", rodeo.resolve(&key));

        let mut rodeo: Rodeo<str, Spur, ahash::RandomState> =
            Rodeo::with_hasher(ahash::RandomState::new());
        let key = rodeo.get_or_intern("Test");
        assert_eq!("Test", rodeo.resolve(&key));
    }

    #[test]
    fn with_capacity_and_hasher() {
        let mut rodeo: Rodeo<str, Spur, RandomState> =
            Rodeo::with_capacity_and_hasher(10, RandomState::new());
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

        let mut rodeo: Rodeo<str, Spur, ahash::RandomState> =
            Rodeo::with_capacity_and_hasher(10, ahash::RandomState::new());
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
        let mut rodeo: Rodeo<str, MicroSpur> = Rodeo::new();

        for i in 0..u8::max_value() as usize - 1 {
            rodeo.get_or_intern(i.to_string());
        }

        let space = rodeo.try_get_or_intern("A").unwrap();
        assert_eq!(Some(space), rodeo.try_get_or_intern("A"));
        assert_eq!("A", rodeo.resolve(&space));

        assert!(rodeo.try_get_or_intern("C").is_none());
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
}
