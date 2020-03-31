use crate::{
    hasher::{HashMap, RandomState},
    key::{Key, Spur},
    reader::RodeoReader,
    resolver::RodeoResolver,
    util::{Iter, Strings},
};

use core::{hash::BuildHasher, mem};

compile! {
    if #[feature = "no_std"] {
        use alloc::{vec::Vec, string::{ToString, String}, boxed::Box};
    }
}

/// A string interner that caches strings quickly with a minimal memory footprint,
/// returning a unique key to re-access it with `O(1)` internment and resolution.
///
/// By default Rodeo uses the [`Spur`] type for keys and [`RandomState`] as its hasher
///
/// [`RandomState`]: index.html#cargo-features
#[derive(Debug)]
pub struct Rodeo<K: Key = Spur, S: BuildHasher + Clone = RandomState> {
    /// Map that allows `str` -> `key` resolution
    map: HashMap<&'static str, K, S>,
    /// Vec that allows `key` -> `str` resolution
    pub(crate) strings: Vec<&'static str>,
}

impl<K: Key> Rodeo<K, RandomState> {
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
        Self {
            map: HashMap::with_hasher(RandomState::new()),
            strings: Vec::new(),
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
    /// let rodeo: Rodeo<Spur> = Rodeo::with_capacity(10);
    /// ```
    ///
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            map: HashMap::with_capacity_and_hasher(capacity, RandomState::new()),
            strings: Vec::with_capacity(capacity),
        }
    }
}

impl<K, S> Rodeo<K, S>
where
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
    /// let rodeo: Rodeo<Spur, RandomState> = Rodeo::with_hasher(RandomState::new());
    /// ```
    ///
    #[inline]
    pub fn with_hasher(hash_builder: S) -> Self {
        Self {
            map: HashMap::with_hasher(hash_builder),
            strings: Vec::new(),
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
    /// let rodeo: Rodeo<Spur, RandomState> = Rodeo::with_capacity_and_hasher(10, RandomState::new());
    /// ```
    ///
    #[inline]
    pub fn with_capacity_and_hasher(capacity: usize, hash_builder: S) -> Self {
        Self {
            map: HashMap::with_capacity_and_hasher(capacity, hash_builder),
            strings: Vec::with_capacity(capacity),
        }
    }

    /// Attempt to intern a string, updating the value if it already exists,
    /// returning its key if the key is able to be made and `None` if not.
    ///
    /// Can be used to determine if another interner needs to be created due to the namespace
    /// of the key being full.  
    /// Determines if the key can be made by using [`Key::try_from_usize`].
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::Rodeo;
    ///
    /// let mut rodeo = Rodeo::default();
    ///
    /// let key = rodeo.try_get_or_intern("Strings of things with wings and dings").unwrap();
    /// assert_eq!("Strings of things with wings and dings", rodeo.resolve(&key));
    /// ```
    ///
    /// [`Key::try_from`]: crate::Key#try_from_usize
    #[inline]
    pub(crate) fn try_intern<T>(&mut self, val: T) -> Option<K>
    where
        T: Into<String>,
    {
        let key = K::try_from_usize(self.strings.len())?;
        let string = Box::leak(val.into().into_boxed_str());

        self.strings.push(string);
        self.map.insert(string, key);

        Some(key)
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
        T: Into<String> + AsRef<str>,
    {
        if let Some(key) = self.get(val.as_ref()) {
            key
        } else {
            self.try_intern(val.into())
                .expect("Failed to get or intern string")
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
        T: Into<String> + AsRef<str>,
    {
        if let Some(key) = self.get(val.as_ref()) {
            Some(key)
        } else {
            self.try_intern(val.into())
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
        T: AsRef<str>,
    {
        self.map.get(val.as_ref()).copied()
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
    /// use lasso::{Spur, Rodeo};
    ///
    /// let rodeo: Rodeo<Spur> = Rodeo::with_capacity(10);
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
    pub fn iter(&self) -> Iter<K> {
        Iter::from_rodeo(self)
    }

    /// Returns an iterator over the interned strings
    #[inline]
    pub fn strings(&self) -> Strings<K> {
        Strings::from_rodeo(self)
    }

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
    pub fn into_reader(mut self) -> RodeoReader<K, S> {
        // Take the strings vec from the old rodeo
        let strings = mem::replace(&mut self.strings, Vec::new());

        // Drain the DashMap by draining each of its buckets and creating a new hashmap to store their values
        let mut map: HashMap<&'static str, K, S> =
            HashMap::with_capacity_and_hasher(strings.len(), self.map.hasher().clone());
        map.extend(self.map.drain());

        // Safety: No other references outside of `map` and `strings` to the interned strings exist
        unsafe { RodeoReader::new(map, strings) }
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
    pub fn into_resolver(mut self) -> RodeoResolver<K> {
        self.map.clear();

        let mut strings = Vec::with_capacity(self.strings.len());

        for string in self.strings.drain(..) {
            strings.push(string);
        }

        // Safety: No other references to the strings exist
        unsafe { RodeoResolver::new(strings) }
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

impl<K, S> Clone for Rodeo<K, S>
where
    K: Key,
    S: BuildHasher + Clone,
{
    #[inline]
    fn clone(&self) -> Self {
        // Safety: The strings of the current Rodeo **cannot** be used in the new one,
        // otherwise it will cause double-frees

        // Create the new map/vec that will fill the new Rodeo, pre-allocating their capacity
        let mut map =
            HashMap::with_capacity_and_hasher(self.strings.len(), self.map.hasher().clone());
        let mut strings = Vec::with_capacity(self.strings.len());

        // For each string in the to-be-cloned Reader, take ownership of each string by calling to_string,
        // therefore cloning it onto the heap, calling into_boxed_str and leaking that
        for (i, string) in self.strings.iter().enumerate() {
            // Clone the static string from self.strings, box and leak it
            let new: &'static str = Box::leak((*string).to_string().into_boxed_str());

            // Store the new string, which we have ownership of, in the new map and vec
            strings.push(new);
            // The indices of the vector correspond with the keys
            map.insert(new, K::try_from_usize(i).unwrap_or_else(|| unreachable!()));
        }

        Self { map, strings }
    }
}

/// Deallocate the leaked strings interned by Rodeo
impl<K, S> Drop for Rodeo<K, S>
where
    K: Key,
    S: BuildHasher + Clone,
{
    #[inline]
    fn drop(&mut self) {
        // Clear the map to remove all other references to the strings in self.strings
        self.map.clear();

        // Drain self.strings while deallocating the strings it holds
        for string in self.strings.drain(..) {
            // Safety: There must not be any other references to the strings being re-boxed, so the
            // map containing all other references is first drained, leaving the sole reference to
            // the strings vector, which allows the safe dropping of the string. This also relies on the
            // implemented functions for Rodeo not giving out any references to the strings it holds
            // that live beyond itself. It also relies on the Clone implementation of Rodeo to clone and
            // take ownership of all the interned strings as to not have a double free when one is dropped
            unsafe {
                let _ = Box::from_raw(string as *const str as *mut str);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{hasher::RandomState, Key, MicroSpur, Rodeo, Spur};

    compile! {
        if #[feature = "no_std"] {
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
        let mut rodeo: Rodeo<Spur> = Rodeo::with_capacity(10);
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

        let mut rodeo: Rodeo<Spur, ahash::RandomState> =
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
    fn try_intern() {
        let mut rodeo: Rodeo<MicroSpur> = Rodeo::new();

        for i in 0..u8::max_value() as usize - 1 {
            rodeo.get_or_intern(i.to_string());
        }

        let space = rodeo.try_intern("A").unwrap();
        assert_eq!("A", rodeo.resolve(&space));

        assert!(rodeo.try_intern("C").is_none());
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

    #[test]
    fn clone_rodeo() {
        let mut rodeo = Rodeo::default();
        let key = rodeo.get_or_intern("Test");

        assert_eq!("Test", rodeo.resolve(&key));

        let cloned = rodeo.clone();
        assert_eq!("Test", cloned.resolve(&key));

        drop(rodeo);

        assert_eq!("Test", cloned.resolve(&key));
    }

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
    #[cfg(not(feature = "no_std"))]
    fn debug() {
        let rodeo = Rodeo::default();
        println!("{:?}", rodeo);
    }
}
