use crate::{
    hasher::{HashMap, RandomState},
    key::{Cord, Key},
    locks::Mutex,
    reader::RodeoReader,
    resolver::RodeoResolver,
};

use core::{hash::BuildHasher, mem};
use dashmap::DashMap;

compile! {
    if #[feature = "no_std"] {
        use alloc::{vec::Vec, string::{ToString, String}, boxed::Box};
    }
}

/// A concurrent string interner that caches strings quickly with a minimal memory footprint,
/// returning a unique key to re-access it with `O(1)` internment and resolution.
///
/// This struct is only avaliable with the `multi-threaded` feature!  
/// By default ThreadedRodeo uses the [`Cord`] type for keys and [`RandomState`] as the hasher
///
/// [`Cord`]: crate::Cord
/// [`ahash::RandomState`]: https://docs.rs/ahash/0.3.2/ahash/struct.RandomState.html
/// [`RandomState`]: index.html#cargo-features
#[derive(Debug)]
pub struct ThreadedRodeo<K: Key = Cord, S: BuildHasher + Clone = RandomState> {
    /// Map that allows str to key resolution
    map: DashMap<&'static str, K, S>,
    /// Vec that allows key to str resolution
    pub(crate) strings: Mutex<Vec<&'static str>>,
}

// TODO: More parity functions with std::HashMap

impl<K: Key> ThreadedRodeo<K, RandomState> {
    /// Create a new ThreadedRodeo
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::{ThreadedRodeo, Cord};
    /// use std::{thread, sync::Arc};
    ///
    /// let lasso: Arc<ThreadedRodeo<Cord>> = Arc::new(ThreadedRodeo::new());
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
    #[inline]
    pub fn new() -> Self {
        Self {
            map: DashMap::with_hasher(RandomState::new()),
            strings: Mutex::new(Vec::new()),
        }
    }

    /// Create a new ThreadedRodeo with the specified capacity. The interner will be able to hold `capacity`
    /// strings without reallocating. If capacity is 0, the interner will not allocate.
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::{ThreadedRodeo, Cord};
    ///
    /// let rodeo: ThreadedRodeo<Cord> = ThreadedRodeo::with_capacity(10);
    /// ```
    ///
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            map: DashMap::with_capacity_and_hasher(capacity, RandomState::new()),
            strings: Mutex::new(Vec::with_capacity(capacity)),
        }
    }
}

impl<K, S> ThreadedRodeo<K, S>
where
    K: Key,
    S: BuildHasher + Clone,
{
    /// Creates an empty ThreadedRodeo which will use the given hasher for its internal hashmap
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::{Cord, ThreadedRodeo};
    /// use std::collections::hash_map::RandomState;
    ///
    /// let rodeo: ThreadedRodeo<Cord, RandomState> = ThreadedRodeo::with_hasher(RandomState::new());
    /// ```
    ///
    #[inline]
    pub fn with_hasher(hash_builder: S) -> Self {
        Self {
            map: DashMap::with_hasher(hash_builder),
            strings: Mutex::new(Vec::new()),
        }
    }

    /// Creates a new ThreadedRodeo with the specified capacity that will use the given hasher for its internal hashmap
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::{Cord, ThreadedRodeo};
    /// use std::collections::hash_map::RandomState;
    ///
    /// let rodeo: ThreadedRodeo<Cord, RandomState> = ThreadedRodeo::with_capacity_and_hasher(10, RandomState::new());
    /// ```
    ///
    #[inline]
    pub fn with_capacity_and_hasher(capacity: usize, hash_builder: S) -> Self {
        Self {
            map: DashMap::with_capacity_and_hasher(capacity, hash_builder),
            strings: Mutex::new(Vec::with_capacity(capacity)),
        }
    }

    /// Intern a string, updating the value if it already exists, and return its key
    ///
    /// # Panics
    ///
    /// Panics if the call to `Key::from_usize` fails, indicating that the key's namespace is full
    ///
    #[inline]
    pub(crate) fn intern<T>(&self, val: T) -> K
    where
        T: Into<String>,
    {
        let (key, string) = {
            #[cfg(feature = "parking_locks")]
            let mut strings = self.strings.lock();
            #[cfg(not(feature = "parking_locks"))]
            let mut strings = self.strings.lock().unwrap();

            let key = K::try_from_usize(strings.len()).expect("The key's namespace is full");
            let string: &'static str = Box::leak(val.into().into_boxed_str());

            strings.push(string);

            (key, string)
        };

        self.map.insert(string, key);

        key
    }

    /// Attempt to intern a string, updating the value if it already exists,
    /// returning its key if the key is able to be made and `None` if not.
    ///
    /// Can be used to determine if another interner needs to be created due to the namespace
    /// of the key being full.  
    /// Determines if the key can be made by using [`Key::try_from_usize`].
    ///
    /// [`Key::try_from`]: crate::Key#try_from_usize
    #[inline]
    pub(crate) fn try_intern<T>(&self, val: T) -> Option<K>
    where
        T: Into<String>,
    {
        let (key, string) = {
            #[cfg(feature = "parking_locks")]
            let mut strings = self.strings.lock();
            #[cfg(not(feature = "parking_locks"))]
            let mut strings = self.strings.lock().unwrap();

            let key = K::try_from_usize(strings.len())?;
            let string: &'static str = Box::leak(val.into().into_boxed_str());

            strings.push(string);

            (key, string)
        };

        self.map.insert(string, key);

        Some(key)
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
    #[inline]
    pub fn get_or_intern<T>(&self, val: T) -> K
    where
        T: Into<String> + AsRef<str>,
    {
        if let Some(key) = self.get(val.as_ref()) {
            key
        } else {
            self.intern(val.into())
        }
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
    #[inline]
    pub fn try_get_or_intern<T>(&self, val: T) -> Option<K>
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
    #[inline]
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
    #[inline]
    pub fn resolve<'a>(&'a self, key: &K) -> &'a str {
        // Safety: The call to get_unchecked's safety relies on the Key::into_usize impl
        // being symmetric and the caller having not fabricated a key. If the impl is sound
        // and symmetric, then it will succeed, as the usize used to create it is a valid
        // index into self.strings
        unsafe {
            #[cfg(feature = "parking_locks")]
            assert!(key.into_usize() < self.strings.lock().len());
            #[cfg(not(feature = "parking_locks"))]
            assert!(key.into_usize() < self.strings.lock().unwrap().len());

            #[cfg(feature = "parking_locks")]
            return self.strings.lock().get_unchecked(key.into_usize());
            #[cfg(not(feature = "parking_locks"))]
            return self.strings.lock().unwrap().get_unchecked(key.into_usize());
        }
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
    #[inline]
    pub fn try_resolve<'a>(&'a self, key: &K) -> Option<&'a str> {
        // Safety: The call to get_unchecked's safety relies on the Key::into_usize impl
        // being symmetric and the caller having not fabricated a key. If the impl is sound
        // and symmetric, then it will succeed, as the usize used to create it is a valid
        // index into self.strings
        unsafe {
            #[cfg(feature = "parking_locks")]
            let in_bounds = key.into_usize() < self.strings.lock().len();
            #[cfg(not(feature = "parking_locks"))]
            let in_bounds = key.into_usize() < self.strings.lock().unwrap().len();

            if in_bounds {
                #[cfg(feature = "parking_locks")]
                return Some(self.strings.lock().get_unchecked(key.into_usize()));
                #[cfg(not(feature = "parking_locks"))]
                return Some(self.strings.lock().unwrap().get_unchecked(key.into_usize()));
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
    /// use lasso::ThreadedRodeo;
    ///
    /// let rodeo = ThreadedRodeo::default();
    ///
    /// let key = rodeo.get_or_intern("Strings of things with wings and dings");
    /// unsafe {
    ///     assert_eq!("Strings of things with wings and dings", rodeo.resolve_unchecked(&key));
    /// }
    /// ```
    ///
    #[inline]
    pub unsafe fn resolve_unchecked<'a>(&'a self, key: &K) -> &'a str {
        #[cfg(feature = "parking_locks")]
        return self.strings.lock().get_unchecked(key.into_usize());

        #[cfg(not(feature = "parking_locks"))]
        return self.strings.lock().unwrap().get_unchecked(key.into_usize());
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
    #[inline]
    pub fn len(&self) -> usize {
        #[cfg(feature = "parking_locks")]
        return self.strings.lock().len();

        #[cfg(not(feature = "parking_locks"))]
        return self.strings.lock().unwrap().len();
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
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the number of strings that can be interned without a reallocation
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::{Cord, ThreadedRodeo};
    ///
    /// let rodeo: ThreadedRodeo<Cord> = ThreadedRodeo::with_capacity(10);
    /// assert_eq!(rodeo.capacity(), 10);
    /// ```
    ///
    #[inline]
    pub fn capacity(&self) -> usize {
        #[cfg(feature = "parking_locks")]
        return self.strings.lock().capacity();

        #[cfg(not(feature = "parking_locks"))]
        return self.strings.lock().unwrap().capacity();
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
    #[inline]
    #[must_use]
    pub fn into_reader(self) -> RodeoReader<K, S> {
        // Take the strings vec from the old lasso
        #[cfg(feature = "parking_locks")]
        let strings = mem::replace(&mut *self.strings.lock(), Vec::new());
        #[cfg(not(feature = "parking_locks"))]
        let strings = mem::replace(&mut *self.strings.lock().unwrap(), Vec::new());

        // Drain the DashMap by draining each of its buckets and creating a new hashmap to store their values
        let mut map: HashMap<&'static str, K, S> =
            HashMap::with_capacity_and_hasher(strings.len(), self.map.hasher().clone());
        for shard in self.map.shards() {
            // Extend the new map by the contents of the shard
            map.extend(shard.write().drain().map(|(k, v)| (k, v.into_inner())));
        }

        // Safety: No other references outside of `map` and `strings` to the interned strings exist
        unsafe { RodeoReader::new(map, strings) }
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
    #[inline]
    #[must_use]
    pub fn into_resolver(self) -> RodeoResolver<K> {
        self.map.clear();

        #[cfg(feature = "parking_locks")]
        let old_strings = &mut *self.strings.lock();
        #[cfg(not(feature = "parking_locks"))]
        let old_strings = &mut *self.strings.lock().unwrap();

        let mut strings = Vec::with_capacity(old_strings.len());

        for string in old_strings.drain(..) {
            strings.push(string);
        }

        // Safety: No other references to the strings exist
        unsafe { RodeoResolver::new(strings) }
    }
}

/// Creates a ThreadedRodeo using [`Cord`] as its key and [`RandomState`] as its hasher
///
/// [`Cord`]: crate::Cord
/// [`RandomState`]: index.html#cargo-features
impl Default for ThreadedRodeo<Cord, RandomState> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K, S> Clone for ThreadedRodeo<K, S>
where
    K: Key,
    S: BuildHasher + Clone,
{
    fn clone(&self) -> Self {
        // Safety: The strings of the current Rodeo **cannot** be used in the new one,
        // otherwise it will cause double-frees

        #[cfg(feature = "parking_locks")]
        let old_strings = &*self.strings.lock();
        #[cfg(not(feature = "parking_locks"))]
        let old_strings = &*self.strings.lock().unwrap();

        // Create the new map/vec that will fill the new ThreadedRodeo, pre-allocating their capacity
        let map = DashMap::with_capacity_and_hasher(old_strings.len(), self.map.hasher().clone());
        let mut strings = Vec::with_capacity(old_strings.len());

        // For each string in the to-be-cloned Rodeo, take ownership of each string by calling to_string,
        // therefore cloning it onto the heap, calling into_boxed_str and leaking that
        for (i, string) in old_strings.into_iter().enumerate() {
            // Clone the static string from old_strings, box and leak it
            let new: &'static str = Box::leak(string.to_string().into_boxed_str());

            // Store the new string, which we have ownership of, in the new map and vec
            strings.push(new);
            // The indices of the vector correspond with the keys
            map.insert(new, K::try_from_usize(i).unwrap_or_else(|| unreachable!()));
        }

        Self {
            map,
            strings: Mutex::new(strings),
        }
    }
}

/// Deallocate the leaked strings interned by ThreadedRodeo
impl<K, S> Drop for ThreadedRodeo<K, S>
where
    K: Key,
    S: BuildHasher + Clone,
{
    fn drop(&mut self) {
        // Clear the map to remove all other references to the strings in self.strings
        self.map.clear();

        #[cfg(feature = "parking_locks")]
        let strings = &mut *self.strings.lock();
        #[cfg(not(feature = "parking_locks"))]
        let strings = &mut *self.strings.lock().unwrap();

        // Drain self.strings while deallocating the strings it holds
        for string in strings.drain(..) {
            // Safety: There must not be any other references to the strings being re-boxed, so the
            // map containing all other references is first drained, leaving the sole reference to
            // the strings vector, which allows the safe dropping of the string. This also relies on the
            // implemented functions for ThreadedRodeo not giving out any references to the strings it holds
            // that live beyond itself. It also relies on the Clone implementation of ThreadedRodeo to clone and
            // take ownership of all the interned strings as to not have a double free when one is dropped
            unsafe {
                let _ = Box::from_raw(string as *const str as *mut str);
            }
        }
    }
}

unsafe impl<K: Key + Sync, S: BuildHasher + Clone + Sync> Sync for ThreadedRodeo<K, S> {}
unsafe impl<K: Key + Send, S: BuildHasher + Clone + Send> Send for ThreadedRodeo<K, S> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{hasher::RandomState, MicroCord};

    #[cfg(not(any(miri, feature = "no_std")))]
    use std::{sync::Arc, thread};

    #[test]
    fn new() {
        let _: ThreadedRodeo<Cord> = ThreadedRodeo::new();
    }

    #[test]
    fn with_capacity() {
        let _: ThreadedRodeo<Cord> = ThreadedRodeo::with_capacity(10);
    }

    #[test]
    fn with_hasher() {
        let std_rodeo: ThreadedRodeo<Cord, RandomState> =
            ThreadedRodeo::with_hasher(RandomState::new());
        let key = std_rodeo.intern("Test");
        assert_eq!("Test", std_rodeo.resolve(&key));
    }

    #[test]
    fn with_capacity_and_hasher() {
        let std_rodeo: ThreadedRodeo<Cord, RandomState> =
            ThreadedRodeo::with_capacity_and_hasher(10, RandomState::new());

        let key = std_rodeo.intern("Test");
        assert_eq!("Test", std_rodeo.resolve(&key));
    }

    #[test]
    fn intern() {
        let rodeo = ThreadedRodeo::default();

        rodeo.intern("A");
        rodeo.intern("A");
        rodeo.intern("B");
        rodeo.intern("B");
        rodeo.intern("C");
        rodeo.intern("C");
    }

    #[test]
    #[cfg(not(any(miri, feature = "no_std")))]
    fn intern_threaded() {
        let rodeo = Arc::new(ThreadedRodeo::default());

        let moved = Arc::clone(&rodeo);
        thread::spawn(move || {
            moved.intern("A");
            moved.intern("A");
            moved.intern("B");
            moved.intern("B");
            moved.intern("C");
            moved.intern("C");
        });

        rodeo.intern("A");
        rodeo.intern("A");
        rodeo.intern("B");
        rodeo.intern("B");
        rodeo.intern("C");
        rodeo.intern("C");
    }

    #[test]
    fn try_intern() {
        let rodeo: ThreadedRodeo<MicroCord> = ThreadedRodeo::new();

        for i in 0..u8::max_value() as usize - 1 {
            rodeo.intern(i.to_string());
        }

        let space = rodeo.try_intern("A").unwrap();
        assert_eq!("A", rodeo.resolve(&space));

        assert!(rodeo.try_intern("C").is_none());
    }

    #[test]
    #[cfg(not(any(miri, feature = "no_std")))]
    fn try_intern_threaded() {
        let rodeo: Arc<ThreadedRodeo<MicroCord>> = Arc::new(ThreadedRodeo::new());

        for i in 0..u8::max_value() as usize - 1 {
            rodeo.intern(i.to_string());
        }

        let space = rodeo.try_intern("A").unwrap();
        assert_eq!("A", rodeo.resolve(&space));

        let moved = Arc::clone(&rodeo);
        thread::spawn(move || {
            assert!(moved.try_intern("C").is_none());
        });

        assert!(rodeo.try_intern("C").is_none());
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
    #[cfg(not(any(miri, feature = "no_std")))]
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
        let rodeo: ThreadedRodeo<MicroCord> = ThreadedRodeo::new();

        for i in 0..u8::max_value() as usize - 1 {
            rodeo.intern(i.to_string());
        }

        let space = rodeo.try_get_or_intern("A").unwrap();
        assert_eq!("A", rodeo.resolve(&space));

        assert!(rodeo.try_get_or_intern("C").is_none());
    }

    #[test]
    #[cfg(not(any(miri, feature = "no_std")))]
    fn try_get_or_intern_threaded() {
        let rodeo: Arc<ThreadedRodeo<MicroCord>> = Arc::new(ThreadedRodeo::new());

        for i in 0..u8::max_value() as usize - 1 {
            rodeo.intern(i.to_string());
        }

        let moved = Arc::clone(&rodeo);
        thread::spawn(move || {
            let a = moved.try_get_or_intern("A");
            assert_eq!(a, moved.try_get_or_intern("A"));

            let b = moved.try_get_or_intern("B");
            assert_eq!(b, moved.try_get_or_intern("B"));

            let c = moved.try_get_or_intern("C");
            assert_eq!(c, moved.try_get_or_intern("C"));
        });

        let a = rodeo.try_get_or_intern("A");
        assert_eq!(a, rodeo.try_get_or_intern("A"));

        let b = rodeo.try_get_or_intern("B");
        assert_eq!(b, rodeo.try_get_or_intern("B"));

        let c = rodeo.try_get_or_intern("C");
        assert_eq!(c, rodeo.try_get_or_intern("C"));
    }

    #[test]
    fn get() {
        let rodeo = ThreadedRodeo::default();
        let key = rodeo.intern("A");

        assert_eq!(Some(key), rodeo.get("A"));
    }

    #[test]
    #[cfg(not(any(miri, feature = "no_std")))]
    fn get_threaded() {
        let rodeo = Arc::new(ThreadedRodeo::default());
        let key = rodeo.intern("A");

        let moved = Arc::clone(&rodeo);
        thread::spawn(move || {
            assert_eq!(Some(key), moved.get("A"));
        });

        assert_eq!(Some(key), rodeo.get("A"));
    }

    #[test]
    fn resolve() {
        let rodeo = ThreadedRodeo::default();
        let key = rodeo.intern("A");

        assert_eq!("A", rodeo.resolve(&key));
    }

    #[test]
    #[should_panic]
    #[cfg(not(miri))]
    fn resolve_panics() {
        let rodeo = ThreadedRodeo::default();
        rodeo.resolve(&Cord::try_from_usize(100).unwrap());
    }

    #[test]
    #[cfg(not(any(miri, feature = "no_std")))]
    fn resolve_threaded() {
        let rodeo = Arc::new(ThreadedRodeo::default());
        let key = rodeo.intern("A");

        let moved = Arc::clone(&rodeo);
        thread::spawn(move || {
            assert_eq!("A", moved.resolve(&key));
        });

        assert_eq!("A", rodeo.resolve(&key));
    }

    #[test]
    fn try_resolve() {
        let rodeo = ThreadedRodeo::default();
        let key = rodeo.intern("A");

        assert_eq!(Some("A"), rodeo.try_resolve(&key));
        assert_eq!(None, rodeo.try_resolve(&Cord::try_from_usize(100).unwrap()));
    }

    #[test]
    #[cfg(not(any(miri, feature = "no_std")))]
    fn try_resolve_threaded() {
        let rodeo = Arc::new(ThreadedRodeo::default());
        let key = rodeo.intern("A");

        let moved = Arc::clone(&rodeo);
        thread::spawn(move || {
            assert_eq!(Some("A"), moved.try_resolve(&key));
            assert_eq!(None, moved.try_resolve(&Cord::try_from_usize(100).unwrap()));
        });

        assert_eq!(Some("A"), rodeo.try_resolve(&key));
        assert_eq!(None, rodeo.try_resolve(&Cord::try_from_usize(100).unwrap()));
    }

    #[test]
    fn resolve_unchecked() {
        let rodeo = ThreadedRodeo::default();
        let key = rodeo.intern("A");

        unsafe {
            assert_eq!("A", rodeo.resolve_unchecked(&key));
        }
    }

    #[test]
    #[cfg(not(any(miri, feature = "no_std")))]
    fn resolve_unchecked_threaded() {
        let rodeo = Arc::new(ThreadedRodeo::default());
        let key = rodeo.intern("A");

        let moved = Arc::clone(&rodeo);
        thread::spawn(move || unsafe {
            assert_eq!("A", moved.resolve_unchecked(&key));
        });

        unsafe {
            assert_eq!("A", rodeo.resolve_unchecked(&key));
        }
    }

    #[test]
    fn len() {
        let rodeo = ThreadedRodeo::default();
        rodeo.intern("A");
        rodeo.intern("B");
        rodeo.intern("C");

        assert_eq!(rodeo.len(), 3);
    }

    #[test]
    fn empty() {
        let rodeo = ThreadedRodeo::default();

        assert!(rodeo.is_empty());
    }

    #[test]
    fn clone() {
        let rodeo = ThreadedRodeo::default();
        let key = rodeo.intern("Test");

        assert_eq!("Test", rodeo.resolve(&key));

        let cloned = rodeo.clone();
        assert_eq!("Test", cloned.resolve(&key));

        drop(rodeo);

        assert_eq!("Test", cloned.resolve(&key));
    }

    #[test]
    fn drops() {
        let _ = ThreadedRodeo::default();
    }

    #[test]
    #[cfg(not(any(miri, feature = "no_std")))]
    fn drop_threaded() {
        let rodeo = Arc::new(ThreadedRodeo::default());

        let moved = Arc::clone(&rodeo);
        thread::spawn(move || {
            let _ = moved;
        });
    }

    #[test]
    #[cfg(not(any(miri, feature = "no_std")))]
    fn debug() {
        let rodeo = ThreadedRodeo::default();
        println!("{:?}", rodeo);
    }
}
