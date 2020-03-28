//! A concurrent string interner that allows strings to be cached with a minimal memory footprint,
//! associating them with a unique [key] that can be used to retrieve them at any time. [`Lassos`] allow `O(1)`
//! internment and resolution and can be turned into a [`ReadOnlyLasso`] to allow for contention-free resolutions
//! with both key to str and str to key operations. It can also be turned into a [`ResolverLasso`] with only
//! key to str operations for the lowest possible memory usage
//!
//! ## Which Interner do I use?
//!
//! No matter which interner you decide to use, you must start with a [`Lasso`], as that is the only way to intern strings and
//! the only way to get the other two interner types. As your program progresses though you may not need to intern strings anymore,
//! and at that point you may choose either a [`ReadOnlyLasso`] or a [`ResolverLasso`]. If you need to go from str to key you
//! should use a [`ReadOnlyLasso`], but anything else should use a [`ResolverLasso`].
//!
//! | Interner          | Thread-safe | Intern String | key to str | str to key | Contention Free | Relative Memory Usage |
//! | ----------------- | :---------: | :-----------: | :--------: | :--------: | :-------------: | :-------------------: |
//! | [`Lasso`]         |      ✅      |       ✅       |     ✅      |     ✅      |        ❌        |         Most          |
//! | [`ReadOnlyLasso`] |      ✅      |       ❌       |     ✅      |     ✅      |        ✅        |        Middle         |
//! | [`ResolverLasso`] |      ✅      |       ❌       |     ✅      |     ❌      |        ✅        |         Least         |
//!
//! ## Example: Interning Strings across threads
//!
//! ```rust
//! use lasso::Lasso;
//! use std::{thread, sync::Arc};
//!
//! let lasso = Arc::new(Lasso::default());
//! let hello = lasso.get_or_intern("Hello, ");
//!
//! let l = Arc::clone(&lasso);
//! let world = thread::spawn(move || {
//!     l.get_or_intern("World!".to_string())
//! })
//! .join()
//! .unwrap();
//!
//! let world_2 = lasso.get_or_intern("World!");
//!
//! assert_eq!("Hello, ", lasso.resolve(&hello));
//! assert_eq!("World!", lasso.resolve(&world));
//!
//! // These are the same because they interned the same string
//! assert_eq!(world, world_2);
//! assert_eq!(lasso.resolve(&world), lasso.resolve(&world_2));
//! ```
//!
//! # Example: Resolving Strings
//!
//! ```rust
//! use lasso::Lasso;
//!
//! let lasso = Lasso::default();
//! let key = lasso.intern("Hello, World!");
//!
//! assert_eq!("Hello, World!", lasso.resolve(&key));
//! ```
//!
//! ## Example: Creating a ReadOnlyLasso
//!
//! ```rust
//! use lasso::Lasso;
//! use std::{thread, sync::Arc};
//!
//! let lasso = Lasso::default();
//! let key = lasso.intern("Contention free!");
//!
//! // Can be used for resolving strings with zero contention, but not for interning new ones
//! let read_only_lasso = Arc::new(lasso.into_read_only());
//!
//! let lasso = Arc::clone(&read_only_lasso);
//! thread::spawn(move || {
//!     assert_eq!("Contention free!", lasso.resolve(&key));
//! });
//!
//! assert_eq!("Contention free!", read_only_lasso.resolve(&key));
//! ```
//!
//! ## Example: Creating a ResolverLasso
//!
//! ```rust
//! use lasso::Lasso;
//! use std::{thread, sync::Arc};
//!
//! let lasso = Lasso::default();
//! let key = lasso.intern("Contention free!");
//!
//! // Can be used for resolving strings with zero contention and the lowest possible memory consumption,
//! // but not for interning new ones
//! let resolver_lasso = Arc::new(lasso.into_resolver());
//!
//! let lasso = Arc::clone(&resolver_lasso);
//! thread::spawn(move || {
//!     assert_eq!("Contention free!", lasso.resolve(&key));
//! });
//!
//! assert_eq!("Contention free!", resolver_lasso.resolve(&key));
//! ```
//!
//! ## Cargo Features
//!
//! * `default` - By default the `ahasher` feature is enabled
//! * `ahasher` - Use [`ahash::RandomState`] as the default hasher for extra speed, without this then std's [`RandomState`] will be used
//!
//! [key]: crate::Key
//! [`Lasso`]: crate::Lasso
//! [`Lassos`]: crate::Lasso
//! [`ReadOnlyLasso`]: crate::ReadOnlyLasso
//! [`ResolverLasso`]: crate::ResolverLasso
//! [`ahash::RandomState`]: https://docs.rs/ahash/0.3.2/ahash/
//! [`RandomState`]: https://doc.rust-lang.org/std/collections/hash_map/struct.RandomState.html

use core::{
    hash::BuildHasher,
    marker::PhantomData,
    mem,
    num::{NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU8, NonZeroUsize},
};
use dashmap::DashMap;
use std::{collections::HashMap, sync::Mutex};

// Enable ahash for the ahasher feature
#[cfg(feature = "ahasher")]
use ahash::RandomState;
#[cfg(not(feature = "ahasher"))]
use std::collections::hash_map::RandomState;

/// A concurrent string interner that caches strings quickly with a minimal memory footprint,
/// returning a unique key to re-access it with `O(1)` internment and resolution.
///
/// By default Lasso uses the [`Cord`] type for keys and `RandomState` as its hasher where `RandomState`
/// is [`ahash::RandomState`] with the `ahasher` feature and std's [`RandomState`] without it
///
/// [`Cord`]: crate::Cord
/// [`ahash::RandomState`]: https://docs.rs/ahash/0.3.2/ahash/struct.RandomState.html
/// [`RandomState`]: https://doc.rust-lang.org/std/collections/hash_map/struct.RandomState.html
#[derive(Debug)]
pub struct Lasso<K: Key = Cord, S: BuildHasher + Clone = RandomState> {
    /// Map that allows `str` -> `key` resolution
    map: DashMap<&'static str, K, S>,
    /// Vec that allows `key` -> `str` resolution
    strings: Mutex<Vec<&'static str>>,
}

impl<K: Key> Lasso<K, RandomState> {
    /// Create a new Lasso
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::{Lasso, Cord};
    /// use std::{thread, sync::Arc};
    ///
    /// let lasso: Arc<Lasso<Cord>> = Arc::new(Lasso::new());
    /// let hello = lasso.intern("Hello, ".to_string());
    ///
    /// let l = Arc::clone(&lasso);
    /// let world = thread::spawn(move || {
    ///     l.intern("World!".to_string())
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

    /// Create a new Lasso with the specified capacity. The interner will be able to hold `capacity`
    /// strings without reallocating. If capacity is 0, the interner will not allocate.
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::{Lasso, Cord};
    ///
    /// let lasso: Lasso<Cord> = Lasso::with_capacity(10);
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

impl<K, S> Lasso<K, S>
where
    K: Key,
    S: BuildHasher + Clone,
{
    /// Creates an empty Lasso which will use the given hasher for its internal hashmap
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::{Cord, Lasso};
    /// use std::collections::hash_map::RandomState;
    ///
    /// let lasso: Lasso<Cord, RandomState> = Lasso::with_hasher(RandomState::new());
    /// ```
    ///
    #[inline]
    pub fn with_hasher(hash_builder: S) -> Self {
        Self {
            map: DashMap::with_hasher(hash_builder),
            strings: Mutex::new(Vec::new()),
        }
    }

    /// Creates a new Lasso with the specified capacity that will use the given hasher for its internal hashmap
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::{Cord, Lasso};
    /// use std::collections::hash_map::RandomState;
    ///
    /// let lasso: Lasso<Cord, RandomState> = Lasso::with_capacity_and_hasher(10, RandomState::new());
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
    /// # Example
    ///
    /// ```rust
    /// use lasso::Lasso;
    ///
    /// let lasso = Lasso::default();
    ///
    /// let key = lasso.intern("Strings of things with wings and dings");
    /// assert_eq!("Strings of things with wings and dings", lasso.resolve(&key));
    /// ```
    ///
    #[inline]
    pub fn intern<T>(&self, val: T) -> K
    where
        T: Into<String>,
    {
        let string = Box::leak(val.into().into_boxed_str());

        let key = {
            let mut strings = self.strings.lock().unwrap();
            let key = K::from_usize(strings.len());
            strings.push(string);

            key
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
    /// # Example
    ///
    /// ```rust
    /// use lasso::Lasso;
    ///
    /// let lasso = Lasso::default();
    ///
    /// let key = lasso.try_intern("Strings of things with wings and dings").unwrap();
    /// assert_eq!("Strings of things with wings and dings", lasso.resolve(&key));
    /// ```
    ///
    /// [`Key::try_from`]: crate::Key#try_from_usize
    #[inline]
    pub fn try_intern<T>(&self, val: T) -> Option<K>
    where
        T: Into<String>,
    {
        let string = Box::leak(val.into().into_boxed_str());

        let key = {
            let mut strings = self.strings.lock().unwrap();
            let key = K::try_from_usize(strings.len())?;
            strings.push(string);

            key
        };

        self.map.insert(string, key);

        Some(key)
    }

    /// Get the key for a string, interning it if it does not yet exist
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::Lasso;
    ///
    /// let lasso = Lasso::default();
    ///
    /// // Interned the string
    /// let key = lasso.get_or_intern("Strings of things with wings and dings");
    /// assert_eq!("Strings of things with wings and dings", lasso.resolve(&key));
    ///
    /// // No string was interned, as it was already contained
    /// let key = lasso.get_or_intern("Strings of things with wings and dings");
    /// assert_eq!("Strings of things with wings and dings", lasso.resolve(&key));
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
    /// use lasso::Lasso;
    ///
    /// let lasso = Lasso::default();
    ///
    /// // Interned the string
    /// let key = lasso.get_or_intern("Strings of things with wings and dings");
    /// assert_eq!("Strings of things with wings and dings", lasso.resolve(&key));
    ///
    /// // No string was interned, as it was already contained
    /// let key = lasso.get_or_intern("Strings of things with wings and dings");
    /// assert_eq!("Strings of things with wings and dings", lasso.resolve(&key));
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
    /// use lasso::{Cord, Lasso};
    ///
    /// let lasso = Lasso::default();
    ///
    /// let key = lasso.intern("Strings of things with wings and dings");
    /// assert_eq!(Some(key), lasso.get("Strings of things with wings and dings"));
    ///
    /// assert_eq!(None, lasso.get("This string isn't interned"));
    /// ```
    ///
    #[inline]
    pub fn get<T>(&self, val: T) -> Option<K>
    where
        T: AsRef<str>,
    {
        self.map.get(val.as_ref()).map(|k| *k)
    }

    /// Resolves a string by its key
    ///
    /// # Safety
    ///
    /// Safety relies on the given [`Key`]'s adherence to symmetry, if the key gives bad indexes then it will cause UB,
    /// With that in mind, **never** use any key not directly provided by the interner
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::{Cord, Key, Lasso};
    ///
    /// let lasso = Lasso::default();
    ///
    /// let key = lasso.intern("Strings of things with wings and dings");
    /// assert_eq!("Strings of things with wings and dings", lasso.resolve(&key));
    /// ```
    ///
    #[inline]
    pub fn resolve<'a>(&'a self, key: &K) -> &'a str {
        unsafe { self.strings.lock().unwrap().get_unchecked(key.into_usize()) }
    }

    /// Gets the number of interned strings
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::Lasso;
    ///
    /// let lasso = Lasso::default();
    /// lasso.intern("Documentation often has little hidden bits in it");
    ///
    /// assert_eq!(lasso.len(), 1);
    /// ```
    ///
    #[inline]
    pub fn len(&self) -> usize {
        self.strings.lock().unwrap().len()
    }

    /// Returns `true` if there are no currently interned strings
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::Lasso;
    ///
    /// let lasso = Lasso::default();
    /// assert!(lasso.is_empty());
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
    /// use lasso::{Cord, Lasso};
    ///
    /// let lasso: Lasso<Cord> = Lasso::with_capacity(10);
    /// assert_eq!(lasso.capacity(), 10);
    /// ```
    ///
    #[inline]
    pub fn capacity(&self) -> usize {
        self.strings.lock().unwrap().capacity()
    }

    /// Consumes the current Lasso, returning a [`ReadOnlyLasso`] to allow contention-free access of the interner
    /// from multiple threads
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::Lasso;
    ///
    /// let lasso = Lasso::default();
    /// let key = lasso.intern("Appear weak when you are strong, and strong when you are weak.");
    ///
    /// let read_only_lasso = lasso.into_read_only();
    /// assert_eq!(
    ///     "Appear weak when you are strong, and strong when you are weak.",
    ///     read_only_lasso.resolve(&key),
    /// );
    /// ```
    ///
    /// [`ReadOnlyLasso`]: crate::ReadOnlyLasso
    #[inline]
    #[must_use]
    pub fn into_read_only(self) -> ReadOnlyLasso<K, S> {
        // Take the strings vec from the old lasso
        let strings = mem::replace(&mut *self.strings.lock().unwrap(), Vec::new());

        // Drain the DashMap by draining each of its buckets and creating a new hashmap to store their values
        let mut map: HashMap<&'static str, K, S> =
            HashMap::with_capacity_and_hasher(strings.len(), self.map.hasher().clone());
        for shard in self.map.shards() {
            // Extend the new map by the contents of the shard
            map.extend(shard.write().drain().map(|(k, v)| (k, v.into_inner())));
        }

        self.map.clear();

        ReadOnlyLasso { map, strings }
    }

    /// Consumes the current Lasso, returning a [`ResolverLasso`] to allow contention-free access of the interner
    /// from multiple threads with the lowest possible memory consumption
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::Lasso;
    ///
    /// let lasso = Lasso::default();
    /// let key = lasso.intern("Appear weak when you are strong, and strong when you are weak.");
    ///
    /// let resolver_lasso = lasso.into_resolver();
    /// assert_eq!(
    ///     "Appear weak when you are strong, and strong when you are weak.",
    ///     resolver_lasso.resolve(&key),
    /// );
    /// ```
    ///
    /// [`ResolverLasso`]: crate::ResolverLasso
    #[inline]
    #[must_use]
    pub fn into_resolver(self) -> ResolverLasso<K> {
        self.map.clear();

        let old_strings = &mut *self.strings.lock().unwrap();
        let mut strings = Vec::with_capacity(old_strings.len());

        for string in old_strings.drain(..) {
            strings.push(string);
        }

        ResolverLasso {
            strings,
            __key: PhantomData,
        }
    }
}

/// Creates a Lasso using [`Cord`] as its key and [`RandomState`] as its hasher
///
/// [`Cord`]: crate::Cord
/// [`RandomState`]: crate#cargo-features
impl Default for Lasso<Cord, RandomState> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K, S> Clone for Lasso<K, S>
where
    K: Key,
    S: BuildHasher + Clone,
{
    fn clone(&self) -> Self {
        // Clone the old strings
        // Safety: THESE STRINGS CANNOT BE USED IN THE NEW LASSO
        // The strings in `old_strings` must be cloned and ownership given
        // to the new Lasso
        let old_strings = &*self.strings.lock().unwrap();

        // Create the new map/vec that will fill the new Lasso
        let map = DashMap::with_capacity_and_hasher(old_strings.len(), self.map.hasher().clone());
        let mut strings = Vec::with_capacity(old_strings.len());

        // For each string in the to-be-cloned Lasso, take ownership of each string and
        // insert it into the map and vec
        for (i, string) in old_strings.into_iter().enumerate() {
            // Clone the static string from `old_strings`, box and leak it
            let new: &'static str = Box::leak(string.to_string().into_boxed_str());

            // Store the new string, which we have ownership of, in the new map and vec
            strings.push(new);
            map.insert(new, K::from_usize(i));
        }

        Self {
            map,
            strings: Mutex::new(strings),
        }
    }
}

/// Deallocate the leaked strings interned by Lasso
impl<K, S> Drop for Lasso<K, S>
where
    K: Key,
    S: BuildHasher + Clone,
{
    fn drop(&mut self) {
        // Clear the map to remove all other references to the strings in self.strings
        self.map.clear();

        // Drain self.strings while deallocating the strings it holds
        for string in self.strings.lock().unwrap().drain(..) {
            // Safety: There must not be any other references to the strings being re-boxed, so the
            // map containing all other references is first drained, leaving the sole reference to
            // the strings vector, which allows the safe dropping of the string. This also relies on the
            // implemented functions for Lasso not giving out any references to the strings it holds
            // that live beyond itself. It also relies on the Clone implementation of Lasso to clone and
            // take ownership of all the interned strings as to not have a double free when one is dropped
            unsafe {
                let _ = Box::from_raw(string as *const str as *mut str);
            }
        }
    }
}

// Safety: Send and Sync are safe, as mutable access to `self.strings` is protected by a `Mutex`
unsafe impl<K: Key, S: BuildHasher + Clone> Send for Lasso<K, S> {}
unsafe impl<K: Key, S: BuildHasher + Clone> Sync for Lasso<K, S> {}

/// A read-only view of a [`Lasso`] that allows contention-free access to interned strings  
///
/// Made with the [`Lasso::into_read_only`] method, the key and hasher types default to the ones used by
/// the [`Lasso`] that created it
///
/// [`Lasso`]: crate::Lasso
/// [`Lasso::into_read_only`]: crate::Lasso#into_read_only
#[derive(Debug)]
pub struct ReadOnlyLasso<K: Key = Cord, S: BuildHasher + Clone = RandomState> {
    /// Map that allows `str` -> `key` resolution
    map: HashMap<&'static str, K, S>,
    /// Vec that allows `key` -> `str` resolution
    strings: Vec<&'static str>,
}

impl<K: Key, S: BuildHasher + Clone> ReadOnlyLasso<K, S> {
    /// Get the key value of a string, returning `None` if it doesn't exist
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::{Cord, Lasso};
    ///
    /// let lasso = Lasso::default();
    /// let key = lasso.intern("Strings of things with wings and dings");
    ///
    /// let lasso = lasso.into_read_only();
    /// assert_eq!(Some(key), lasso.get("Strings of things with wings and dings"));
    ///
    /// assert_eq!(None, lasso.get("This string isn't interned"));
    /// ```
    ///
    #[inline]
    pub fn get<T>(&self, val: T) -> Option<K>
    where
        T: AsRef<str>,
    {
        self.map.get(val.as_ref()).map(|k| *k)
    }

    /// Resolves a string by its key
    ///
    /// # Safety
    ///
    /// Safety relies on the given [`Key`]'s adherence to symmetry, if the key gives bad indexes then it will cause UB.  
    /// With that in mind, **never** use any key not directly provided by the interner
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::{Cord, Key, Lasso};
    ///
    /// let lasso = Lasso::default();
    /// let key = lasso.intern("Strings of things with wings and dings");
    ///
    /// let lasso = lasso.into_read_only();
    /// assert_eq!("Strings of things with wings and dings", lasso.resolve(&key));
    /// ```
    ///
    #[inline]
    pub fn resolve<'a>(&'a self, key: &K) -> &'a str {
        unsafe { self.strings.get_unchecked(key.into_usize()) }
    }

    /// Gets the number of interned strings
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::Lasso;
    ///
    /// let lasso = Lasso::default();
    /// lasso.intern("Documentation often has little hidden bits in it");
    ///
    /// let lasso = lasso.into_read_only();
    /// assert_eq!(lasso.len(), 1);
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
    /// use lasso::Lasso;
    ///
    /// let lasso = Lasso::default();
    ///
    /// let lasso = lasso.into_read_only();
    /// assert!(lasso.is_empty());
    /// ```
    ///
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Consumes the current ReadOnlyLasso, making it into a [`ResolverLasso`], allowing
    /// contention-free access from multiple threads with the lowest possible memory consumption
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::Lasso;
    ///
    /// let lasso = Lasso::default();
    /// let key = lasso.intern("Appear weak when you are strong, and strong when you are weak.");
    /// let read_only_lasso = lasso.into_read_only();
    ///
    /// let resolver_lasso = read_only_lasso.into_resolver();
    /// assert_eq!(
    ///     "Appear weak when you are strong, and strong when you are weak.",
    ///     resolver_lasso.resolve(&key),
    /// );
    /// ```
    ///
    /// [`ResolverLasso`]: crate::ResolverLasso
    #[inline]
    #[must_use]
    pub fn into_resolver(mut self) -> ResolverLasso<K> {
        let strings = mem::take(&mut self.strings);

        ResolverLasso {
            strings,
            __key: PhantomData,
        }
    }
}

impl<K, S> Clone for ReadOnlyLasso<K, S>
where
    K: Key,
    S: BuildHasher + Clone,
{
    fn clone(&self) -> Self {
        // Safety: The strings of the current ReadOnlyLasso **cannot** be used in the new lasso

        // Create the new map/vec that will fill the new Lasso
        let mut map =
            HashMap::with_capacity_and_hasher(self.strings.len(), self.map.hasher().clone());
        let mut strings = Vec::with_capacity(self.strings.len());

        // For each string in the to-be-cloned Lasso, take ownership of each string and
        // insert it into the map and vec
        for (i, string) in self.map.keys().enumerate() {
            // Clone the static string from `old_strings` onto the heap, box and leak it
            let new: &'static str = Box::leak(string.to_string().into_boxed_str());

            // Store the new string, which we have ownership of, in the new map and vec
            strings.push(new);
            map.insert(new, K::from_usize(i));
        }

        Self { map, strings }
    }
}

/// Deallocate the leaked strings interned by ReadOnlyLasso
impl<K: Key, S: BuildHasher + Clone> Drop for ReadOnlyLasso<K, S> {
    fn drop(&mut self) {
        // Clear the map to remove all other references to the strings in self.strings
        self.map.clear();

        // Drain self.strings while deallocating the strings it holds
        for string in self.strings.drain(..) {
            // Safety: There must not be any other references to the strings being re-boxed, so the
            // map containing all other references is first drained, leaving the sole reference to
            // the strings vector, which allows the safe dropping of the string. This also relies on the
            // implemented functions for ReadOnlyLasso not giving out any references to the strings it holds
            // that live beyond itself. It also relies on the Clone implementation of ReadOnlyLasso to clone and
            // take ownership of all the interned strings as to not have a double free when one is dropped
            unsafe {
                let _ = Box::from_raw(string as *const str as *mut str);
            }
        }
    }
}

// Safety: Send and Sync are safe, as mutable access is not possible
unsafe impl<K: Key, S: BuildHasher + Clone> Send for ReadOnlyLasso<K, S> {}
unsafe impl<K: Key, S: BuildHasher + Clone> Sync for ReadOnlyLasso<K, S> {}

/// A read-only view of a [`Lasso`] that allows contention-free access to interned strings, but only for `key` -> `str`
/// resolution
///
/// Made with the [`Lasso::into_resolver`] method, the key type defaults to the ones used by the [`Lasso`] that created it
///
/// [`Lasso`]: crate::Lasso
/// [`Lasso::into_resolver`]: crate::Lasso#into_resolver
#[derive(Debug)]
pub struct ResolverLasso<K: Key = Cord> {
    /// Vec that allows `key` -> `str` resolution
    strings: Vec<&'static str>,
    __key: PhantomData<K>,
}

impl<K: Key> ResolverLasso<K> {
    /// Resolves a string by its key
    ///
    /// # Safety
    ///
    /// Safety relies on the given [`Key`]'s adherence to symmetry, if the key gives bad indexes then it will cause UB,
    /// With that in mind, **never** use any key not directly provided by the interner
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::{Cord, Key, Lasso};
    ///
    /// let lasso = Lasso::default();
    /// let key = lasso.intern("Strings of things with wings and dings");
    ///
    /// let lasso = lasso.into_resolver();
    /// assert_eq!("Strings of things with wings and dings", lasso.resolve(&key));
    /// ```
    ///
    /// [`Key`]: crate::Key
    #[inline]
    pub fn resolve<'a>(&'a self, key: &K) -> &'a str {
        unsafe { self.strings.get_unchecked(key.into_usize()) }
    }

    /// Gets the number of interned strings
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::Lasso;
    ///
    /// let lasso = Lasso::default();
    /// lasso.intern("Documentation often has little hidden bits in it");
    ///
    /// let lasso = lasso.into_resolver();
    /// assert_eq!(lasso.len(), 1);
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
    /// use lasso::Lasso;
    ///
    /// let lasso = Lasso::default();
    ///
    /// let lasso = lasso.into_resolver();
    /// assert!(lasso.is_empty());
    /// ```
    ///
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<K: Key> Clone for ResolverLasso<K> {
    fn clone(&self) -> Self {
        // Safety: The strings of the current Resolver **cannot** be used in the new one

        // Create the new vec that will fill the new Resolver
        let mut strings = Vec::with_capacity(self.strings.len());

        // For each string in the to-be-cloned Resolver, take ownership of each string and
        // insert it into the new vec
        for string in self.strings.iter() {
            // Clone the static string from `old_strings` onto the heap, box and leak it
            let new: &'static str = Box::leak(string.to_string().into_boxed_str());

            // Store the new string, which we have ownership of, in the new vec
            strings.push(new);
        }

        Self {
            strings,
            __key: PhantomData,
        }
    }
}

/// Deallocate the leaked strings interned by ResolverLasso
impl<K: Key> Drop for ResolverLasso<K> {
    fn drop(&mut self) {
        // Drain self.strings while deallocating the strings it holds
        for string in self.strings.drain(..) {
            // Safety: There must not be any other references to the strings being re-boxed, which lies in the
            // the strings vector, which in turn allows the safe dropping of the string. This also relies on the
            // implemented functions for ResolverLasso not giving out any references to the strings it holds
            // that live beyond itself. It also relies on the Clone implementation of ResolverLasso to clone and
            // take ownership of all the interned strings as to not have a double free when one is dropped
            unsafe {
                let _ = Box::from_raw(string as *const str as *mut str);
            }
        }
    }
}

// Safety: Send and Sync are safe, as mutable access is not possible
unsafe impl<K: Key> Send for ResolverLasso<K> {}
unsafe impl<K: Key> Sync for ResolverLasso<K> {}

/// Types implementing this trait can be used as keys for [`Lasso`]
///
/// # Safety
///
/// into/from must be perfectly symmetrical, any key that goes on must be perfectly reproduced with the other
///
/// [`Lasso`]: crate::Lasso
/// [`ReadOnlyLasso`]: crate::ReadOnlyLasso
pub unsafe trait Key: Copy + Eq {
    /// Returns the `usize` that represents the current key
    fn into_usize(self) -> usize;
    /// Creates a key from a `usize`
    fn from_usize(int: usize) -> Self;
    /// Attempts to create a key from a `usize`, returning `None` if it fails
    fn try_from_usize(int: usize) -> Option<Self>;
}

/// The default key for [`Lasso`].  
///
/// Internally is a `NonZeroUsize` to allow for space optimizations when stored inside of an [`Option`]
///
/// [`Lasso`]: crate::Lasso
/// [`ReadOnlyLasso`]: crate::ReadOnlyLasso
/// [`Option`]: https://doc.rust-lang.org/std/option/enum.Option.html   
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Cord(NonZeroUsize);

unsafe impl Key for Cord {
    #[inline]
    fn into_usize(self) -> usize {
        self.0.get() - 1
    }

    /// Panics if `int` is greater than `usize::MAX - 1`
    #[inline]
    fn from_usize(int: usize) -> Self {
        assert!(
            int < usize::max_value(),
            "Can only use values up to `usize::MAX - 1` for Cord"
        );

        Self(NonZeroUsize::new(int + 1).unwrap_or_else(|| {
            unreachable!("The value is already asserted to be less than usize::MAX")
        }))
    }

    /// Returns `None` if `int` is greater than `usize::MAX - 1`
    #[inline]
    fn try_from_usize(int: usize) -> Option<Self> {
        if int < usize::max_value() {
            Some(Self(NonZeroUsize::new(int + 1).unwrap_or_else(|| {
                unreachable!("The value is already asserted to be less than usize::MAX")
            })))
        } else {
            None
        }
    }
}

/// A small Key for [`Lasso`]
///
/// Internally is a `NonZeroU32` to allow for space optimizations when stored inside of an [`Option`]
///
/// [`Lasso`]: crate::Lasso
/// [`ReadOnlyLasso`]: crate::ReadOnlyLasso
/// [`Option`]: https://doc.rust-lang.org/std/option/enum.Option.html   
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct SmallCord(NonZeroU32);

unsafe impl Key for SmallCord {
    #[inline]
    fn into_usize(self) -> usize {
        self.0.get() as usize - 1
    }

    /// Panics if `int` is greater than `usize::MAX - 1`
    #[inline]
    fn from_usize(int: usize) -> Self {
        assert!(
            int < u32::max_value() as usize,
            "Can only use values up to `u32::MAX - 1` for SmallCord"
        );

        Self(NonZeroU32::new(int as u32 + 1).unwrap_or_else(|| {
            unreachable!("The value is already asserted to be less than u32::MAX")
        }))
    }

    /// Returns `None` if `int` is greater than `usize::MAX - 1`
    #[inline]
    fn try_from_usize(int: usize) -> Option<Self> {
        if int < u32::max_value() as usize {
            Some(Self(NonZeroU32::new(int as u32 + 1).unwrap_or_else(|| {
                unreachable!("The value is already asserted to be less than u32::MAX")
            })))
        } else {
            None
        }
    }
}

macro_rules! impl_key {
    (ints: $($ty:ty),*) => {
        $(
            unsafe impl Key for $ty {
                #[inline]
                fn into_usize(self) -> usize {
                    self as usize
                }

                #[inline]
                fn from_usize(int: usize) -> Self {
                    assert!(
                        int <= <$ty>::max_value() as usize,
                        concat!("Can only use values up to `", stringify!($ty), "::MAX`")
                    );

                    int as $ty
                }

                #[inline]
                fn try_from_usize(int: usize) -> Option<Self> {
                    if int <= <$ty>::max_value() as usize {
                        Some(int as $ty)
                    } else {
                        None
                    }
                }
            }
        )*
    };

    (nonzero: $(($ty:ty, $int:ty)),*) => {
        $(
            unsafe impl Key for $ty {
                #[inline]
                fn into_usize(self) -> usize {
                    self.get() as usize - 1
                }

                #[inline]
                fn from_usize(int: usize) -> Self {
                    assert!(
                        int < <$int>::max_value() as usize,
                        concat!("Can only use values up to `", stringify!($int), "::MAX - 1`"),
                    );

                    <$ty>::new(int as $int + 1).unwrap_or_else(|| {
                        unreachable!(concat!("The value is already asserted to be less than ", stringify!($int), "::MAX"))
                    })
                }

                #[inline]
                fn try_from_usize(int: usize) -> Option<Self> {
                    if int < <$int>::max_value() as usize {
                        Some(<$ty>::new(int as $int + 1).unwrap_or_else(|| {
                            unreachable!(concat!("The value is already asserted to be less than ", stringify!($int), "::MAX"))
                        }))
                    } else {
                        None
                    }
                }
            }
        )*

    };
}

// Implement Key for unsigned integer types, nonzero and normal, up to u64
impl_key!(ints: u8, u16, u32, u64, usize);
impl_key!(
    nonzero: (NonZeroU8, u8),
    (NonZeroU16, u16),
    (NonZeroU32, u32),
    (NonZeroU64, u64),
    (NonZeroUsize, usize)
);

#[cfg(test)]
mod tests {
    use super::*;
    use ahash::RandomState as AhashRandomState;
    use std::collections::hash_map::RandomState as StdRandomState;
    #[cfg(not(miri))]
    use std::{sync::Arc, thread};

    #[test]
    fn lasso_new() {
        let _: Lasso<Cord> = Lasso::new();
    }

    #[test]
    fn lasso_with_capacity() {
        let _: Lasso<Cord> = Lasso::with_capacity(10);
    }

    #[test]
    fn lasso_with_hasher() {
        let std_lasso: Lasso<Cord, StdRandomState> = Lasso::with_hasher(StdRandomState::new());
        let key = std_lasso.intern("Test");
        assert_eq!("Test", std_lasso.resolve(&key));

        let ahash_lasso: Lasso<Cord, AhashRandomState> =
            Lasso::with_hasher(AhashRandomState::new());
        let key = ahash_lasso.intern("Test");
        assert_eq!("Test", ahash_lasso.resolve(&key));
    }

    #[test]
    fn lasso_with_capacity_and_hasher() {
        let std_lasso: Lasso<Cord, StdRandomState> =
            Lasso::with_capacity_and_hasher(10, StdRandomState::new());

        let key = std_lasso.intern("Test");
        assert_eq!("Test", std_lasso.resolve(&key));

        let ahash_lasso: Lasso<Cord, AhashRandomState> =
            Lasso::with_capacity_and_hasher(10, AhashRandomState::new());

        let key = ahash_lasso.intern("Test");
        assert_eq!("Test", ahash_lasso.resolve(&key));
    }

    #[test]
    fn lasso_intern() {
        let lasso = Lasso::default();
        lasso.intern("A");
        lasso.intern("A");
        lasso.intern("B");
        lasso.intern("B");
        lasso.intern("C");
        lasso.intern("C");
    }

    #[test]
    #[cfg(not(miri))]
    fn lasso_intern_threaded() {
        let lasso = Arc::new(Lasso::default());

        let moved = Arc::clone(&lasso);
        thread::spawn(move || {
            moved.intern("A");
            moved.intern("A");
            moved.intern("B");
            moved.intern("B");
            moved.intern("C");
            moved.intern("C");
        });

        lasso.intern("A");
        lasso.intern("A");
        lasso.intern("B");
        lasso.intern("B");
        lasso.intern("C");
        lasso.intern("C");
    }

    #[test]
    fn lasso_get_or_intern() {
        let lasso = Lasso::default();
        lasso.get_or_intern("A");
        lasso.get_or_intern("A");
        lasso.get_or_intern("B");
        lasso.get_or_intern("B");
        lasso.get_or_intern("C");
        lasso.get_or_intern("C");
    }

    #[test]
    #[cfg(not(miri))]
    fn lasso_get_or_intern_threaded() {
        let lasso = Arc::new(Lasso::default());

        let moved = Arc::clone(&lasso);
        thread::spawn(move || {
            moved.get_or_intern("A");
            moved.get_or_intern("A");
            moved.get_or_intern("B");
            moved.get_or_intern("B");
            moved.get_or_intern("C");
            moved.get_or_intern("C");
        });

        lasso.get_or_intern("A");
        lasso.get_or_intern("A");
        lasso.get_or_intern("B");
        lasso.get_or_intern("B");
        lasso.get_or_intern("C");
        lasso.get_or_intern("C");
    }

    #[test]
    fn lasso_get() {
        let lasso = Lasso::default();
        let key = lasso.intern("A");

        assert_eq!(Some(key), lasso.get("A"));
    }

    #[test]
    #[cfg(not(miri))]
    fn lasso_get_threaded() {
        let lasso = Arc::new(Lasso::default());
        let key = lasso.intern("A");

        let moved = Arc::clone(&lasso);
        thread::spawn(move || {
            assert_eq!(Some(key), moved.get("A"));
        });

        assert_eq!(Some(key), lasso.get("A"));
    }

    #[test]
    fn lasso_resolve() {
        let lasso = Lasso::default();
        let key = lasso.intern("A");

        assert_eq!("A", lasso.resolve(&key));
    }

    #[test]
    #[cfg(not(miri))]
    fn lasso_resolve_threaded() {
        let lasso = Arc::new(Lasso::default());
        let key = lasso.intern("A");

        let moved = Arc::clone(&lasso);
        thread::spawn(move || {
            assert_eq!("A", moved.resolve(&key));
        });

        assert_eq!("A", lasso.resolve(&key));
    }

    #[test]
    fn lasso_len() {
        let lasso = Lasso::default();
        lasso.intern("A");
        lasso.intern("B");
        lasso.intern("C");

        assert_eq!(lasso.len(), 3);
    }

    #[test]
    fn lasso_empty() {
        let lasso = Lasso::default();

        assert!(lasso.is_empty());
    }

    #[test]
    fn clone_lasso() {
        let lasso = Lasso::default();
        let key = lasso.intern("Test");

        assert_eq!("Test", lasso.resolve(&key));

        let cloned = lasso.clone();
        assert_eq!("Test", cloned.resolve(&key));

        drop(lasso);

        assert_eq!("Test", cloned.resolve(&key));
    }

    #[test]
    fn drop_lasso() {
        let _ = Lasso::default();
    }

    #[test]
    #[cfg(not(miri))]
    fn drop_lasso_threaded() {
        let lasso = Arc::new(Lasso::default());

        let moved = Arc::clone(&lasso);
        thread::spawn(move || {
            let _ = moved;
        });
    }

    #[test]
    fn read_only_get() {
        let lasso = Lasso::default();
        let key = lasso.intern("A");

        let read_only = lasso.into_read_only();
        assert_eq!(Some(key), read_only.get("A"));
    }

    #[test]
    #[cfg(not(miri))]
    fn read_only_get_threaded() {
        let lasso = Lasso::default();
        let key = lasso.intern("A");

        let read_only = Arc::new(lasso.into_read_only());

        let moved = Arc::clone(&read_only);
        thread::spawn(move || {
            assert_eq!(Some(key), moved.get("A"));
        });

        assert_eq!(Some(key), read_only.get("A"));
    }

    #[test]
    fn read_only_resolve() {
        let lasso = Lasso::default();
        let key = lasso.intern("A");

        let read_only = lasso.into_read_only();
        assert_eq!("A", read_only.resolve(&key));
    }

    #[test]
    #[cfg(not(miri))]
    fn read_only_resolve_threaded() {
        let lasso = Lasso::default();
        let key = lasso.intern("A");

        let read_only = Arc::new(lasso.into_read_only());

        let moved = Arc::clone(&read_only);
        thread::spawn(move || {
            assert_eq!("A", moved.resolve(&key));
        });

        assert_eq!("A", read_only.resolve(&key));
    }

    #[test]
    fn read_only_len() {
        let lasso = Lasso::default();
        lasso.intern("A");
        lasso.intern("B");
        lasso.intern("C");

        let read_only = lasso.into_read_only();
        assert_eq!(read_only.len(), 3);
    }

    #[test]
    fn read_only_empty() {
        let lasso = Lasso::default();
        let read_only = lasso.into_read_only();

        assert!(read_only.is_empty());
    }

    #[test]
    fn clone_read_only() {
        let lasso = Lasso::default();
        let key = lasso.intern("Test");

        let read_only_lasso = lasso.into_read_only();
        assert_eq!("Test", read_only_lasso.resolve(&key));

        let cloned = read_only_lasso.clone();
        assert_eq!("Test", cloned.resolve(&key));

        drop(read_only_lasso);

        assert_eq!("Test", cloned.resolve(&key));
    }

    #[test]
    fn drop_read_only() {
        let lasso = Lasso::default();
        let _ = lasso.into_read_only();
    }

    #[test]
    #[cfg(not(miri))]
    fn drop_read_only_threaded() {
        let lasso = Lasso::default();
        let read_only = Arc::new(lasso.into_read_only());

        let moved = Arc::clone(&read_only);
        thread::spawn(move || {
            let _ = moved;
        });
    }

    #[test]
    fn read_only_into_resolver() {
        let lasso = Lasso::default();
        let key = lasso.intern("A");

        let resolver = lasso.into_read_only().into_resolver();
        assert_eq!("A", resolver.resolve(&key));
    }

    #[test]
    fn resolver_resolve() {
        let lasso = Lasso::default();
        let key = lasso.intern("A");

        let resolver = lasso.into_resolver();
        assert_eq!("A", resolver.resolve(&key));
    }

    #[test]
    #[cfg(not(miri))]
    fn resolver_resolve_threaded() {
        let lasso = Lasso::default();
        let key = lasso.intern("A");

        let resolver = Arc::new(lasso.into_resolver());

        let moved = Arc::clone(&resolver);
        thread::spawn(move || {
            assert_eq!("A", moved.resolve(&key));
        });

        assert_eq!("A", resolver.resolve(&key));
    }

    #[test]
    fn resolver_len() {
        let lasso = Lasso::default();
        lasso.intern("A");
        lasso.intern("B");
        lasso.intern("C");

        let resolver = lasso.into_resolver();
        assert_eq!(resolver.len(), 3);
    }

    #[test]
    fn resolver_empty() {
        let lasso = Lasso::default();
        let read_only = lasso.into_resolver();

        assert!(read_only.is_empty());
    }

    #[test]
    fn clone_resolver() {
        let lasso = Lasso::default();
        let key = lasso.intern("Test");

        let resolver_lasso = lasso.into_resolver();
        assert_eq!("Test", resolver_lasso.resolve(&key));

        let cloned = resolver_lasso.clone();
        assert_eq!("Test", cloned.resolve(&key));

        drop(resolver_lasso);

        assert_eq!("Test", cloned.resolve(&key));
    }

    #[test]
    fn drop_resolver() {
        let lasso = Lasso::default();
        let _ = lasso.into_resolver();
    }

    #[test]
    #[cfg(not(miri))]
    fn drop_resolver_threaded() {
        let lasso = Lasso::default();
        let resolver = Arc::new(lasso.into_resolver());

        let moved = Arc::clone(&resolver);
        thread::spawn(move || {
            let _ = moved;
        });
    }

    #[test]
    fn cord() {
        let zero = Cord::from_usize(0);
        let max = Cord::from_usize(usize::max_value() - 1);

        assert_eq!(zero.into_usize(), 0);
        assert_eq!(max.into_usize(), usize::max_value() - 1);
    }

    #[test]
    fn cord_max_returns_none() {
        assert_eq!(None, Cord::try_from_usize(usize::max_value()));
    }

    #[test]
    #[should_panic]
    #[cfg(not(miri))]
    fn cord_max_panics() {
        Cord::from_usize(usize::max_value());
    }

    #[test]
    fn small_cord() {
        let zero = SmallCord::from_usize(0);
        let max = SmallCord::from_usize(u32::max_value() as usize - 1);

        assert_eq!(zero.into_usize(), 0);
        assert_eq!(max.into_usize(), u32::max_value() as usize - 1);
    }

    #[test]
    fn small_cord_returns_none() {
        assert_eq!(None, SmallCord::try_from_usize(u32::max_value() as usize));
    }

    #[test]
    #[should_panic]
    #[cfg(not(miri))]
    fn small_cord_panics() {
        SmallCord::from_usize(u32::max_value() as usize);
    }
}
