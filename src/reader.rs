use crate::{
    arena::Arena,
    hasher::RandomState,
    key::{Key, Spur},
    resolver::RodeoResolver,
    util::{Iter, Strings},
};
use core::{
    fmt::{Debug, Formatter, Result as FmtResult},
    hash::{BuildHasher, Hash, Hasher},
};
use hashbrown::HashMap;

compile! {
    if #[feature = "no-std"] {
        use alloc::vec::Vec;
    }
}

/// A read-only view of a [`Rodeo`] or [`ThreadedRodeo`] that allows contention-free access to interned strings,
/// both key to string resolution and string to key lookups
///
/// The key and hasher types are the same as the `Rodeo` or `ThreadedRodeo` that created it, can be acquired with the
/// `into_reader` methods.
///
/// [`Rodeo`]: crate::Rodeo
/// [`ThreadedRodeo`]: crate::ThreadedRodeo
pub struct RodeoReader<K = Spur, S = RandomState> {
    // The logic behind this arrangement is more heavily documented inside of
    // `Rodeo` itself
    map: HashMap<K, (), ()>,
    hasher: S,
    pub(crate) strings: Vec<&'static str>,
    arena: Arena,
}

impl<K, S> RodeoReader<K, S> {
    /// Creates a new RodeoReader
    ///
    /// # Safety
    ///
    /// The references inside of `strings` must be absolutely unique, meaning
    /// that no other references to those strings exist
    ///
    pub(crate) unsafe fn new(
        map: HashMap<K, (), ()>,
        hasher: S,
        strings: Vec<&'static str>,
        arena: Arena,
    ) -> Self {
        Self {
            map,
            hasher,
            strings,
            arena,
        }
    }

    /// Get the key value of a string, returning `None` if it doesn't exist
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::Rodeo;
    ///
    /// // ThreadedRodeo is interchangeable for Rodeo here
    /// let mut rodeo = Rodeo::default();
    /// let key = rodeo.get_or_intern("Strings of things with wings and dings");
    ///
    /// let rodeo = rodeo.into_reader();
    /// assert_eq!(Some(key), rodeo.get("Strings of things with wings and dings"));
    ///
    /// assert_eq!(None, rodeo.get("This string isn't interned"));
    /// ```
    ///
    #[inline]
    pub fn get<T>(&self, val: T) -> Option<K>
    where
        T: AsRef<str>,
        S: BuildHasher,
        K: Key,
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

            // Compare the requested string against the key's string
            string_slice == key_string
        });

        entry.map(|(key, ())| *key)
    }

    /// Resolves a string by its key. Only keys made by the current Resolver or the creator
    /// of the current Resolver may be used
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
    /// // ThreadedRodeo is interchangeable for Rodeo here
    /// let mut rodeo = Rodeo::default();
    /// let key = rodeo.get_or_intern("Strings of things with wings and dings");
    ///
    /// let rodeo = rodeo.into_resolver();
    /// assert_eq!("Strings of things with wings and dings", rodeo.resolve(&key));
    /// ```
    ///
    /// [`Key`]: crate::Key
    #[inline]
    pub fn resolve<'a>(&'a self, key: &K) -> &'a str
    where
        K: Key,
    {
        // Safety: The call to get_unchecked's safety relies on the Key::into_usize impl
        // being symmetric and the caller having not fabricated a key. If the impl is sound
        // and symmetric, then it will succeed, as the usize used to create it is a valid
        // index into self.strings
        unsafe {
            assert!(key.into_usize() < self.strings.len());
            self.strings.get_unchecked(key.into_usize())
        }
    }

    /// Resolves a string by its key, returning `None` if the key is out of bounds. Only keys
    /// made by the current Resolver or the creator of the current Resolver may be used
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::Rodeo;
    ///
    /// // ThreadedRodeo is interchangeable for Rodeo here
    /// let mut rodeo = Rodeo::default();
    /// let key = rodeo.get_or_intern("Strings of things with wings and dings");
    ///
    /// let rodeo = rodeo.into_resolver();
    /// assert_eq!(Some("Strings of things with wings and dings"), rodeo.try_resolve(&key));
    /// ```
    ///
    /// [`Key`]: crate::Key
    #[inline]
    pub fn try_resolve<'a>(&'a self, key: &K) -> Option<&'a str>
    where
        K: Key,
    {
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

    /// Resolves a string by its key without bounds checks
    ///
    /// # Safety
    ///
    /// The key must be valid for the current Reader
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::Rodeo;
    ///
    /// // ThreadedRodeo is interchangeable for Rodeo here
    /// let mut rodeo = Rodeo::default();
    /// let key = rodeo.get_or_intern("Strings of things with wings and dings");
    ///
    /// let rodeo = rodeo.into_resolver();
    /// unsafe {
    ///     assert_eq!("Strings of things with wings and dings", rodeo.resolve_unchecked(&key));
    /// }
    /// ```
    ///
    /// [`Key`]: crate::Key
    #[inline]
    pub unsafe fn resolve_unchecked<'a>(&'a self, key: &K) -> &'a str
    where
        K: Key,
    {
        self.strings.get_unchecked(key.into_usize())
    }

    /// Gets the number of interned strings
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::Rodeo;
    ///
    /// // ThreadedRodeo is interchangeable for Rodeo here
    /// let mut rodeo = Rodeo::default();
    /// rodeo.get_or_intern("Documentation often has little hidden bits in it");
    ///
    /// let rodeo = rodeo.into_reader();
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
    /// // ThreadedRodeo is interchangeable for Rodeo here
    /// let rodeo = Rodeo::default();
    ///
    /// let rodeo = rodeo.into_reader();
    /// assert!(rodeo.is_empty());
    /// ```
    ///
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns an iterator over the interned strings and their key values
    #[inline]
    pub fn iter(&self) -> Iter<'_, K> {
        Iter::from_reader(self)
    }

    /// Returns an iterator over the interned strings
    #[inline]
    pub fn strings(&self) -> Strings<'_, K> {
        Strings::from_reader(self)
    }

    /// Consumes the current rodeo, making it into a [`RodeoResolver`], allowing
    /// contention-free access from multiple threads with the lowest possible memory consumption
    ///
    /// # Example
    ///
    /// ```rust
    /// use lasso::Rodeo;
    ///
    /// // ThreadedRodeo is interchangeable for Rodeo here
    /// let mut rodeo = Rodeo::default();
    /// let key = rodeo.get_or_intern("Appear weak when you are strong, and strong when you are weak.");
    /// let reader_rodeo = rodeo.into_reader();
    ///
    /// let resolver_rodeo = reader_rodeo.into_resolver();
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
        let RodeoReader { strings, arena, .. } = self;

        // Safety: The current reader no longer contains references to the strings
        // in the vec given to RodeoResolver
        unsafe { RodeoResolver::new(strings, arena) }
    }
}

impl<K: Debug, S> Debug for RodeoReader<K, S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_struct("Rodeo")
            .field("map", &self.map)
            .field("strings", &self.strings)
            .finish()
    }
}

unsafe impl<K: Sync, S: Sync> Sync for RodeoReader<K, S> {}
unsafe impl<K: Send, S: Send> Send for RodeoReader<K, S> {}

#[cfg(test)]
mod tests {
    mod single_threaded {
        use crate::{single_threaded::Rodeo, Key, Spur};

        #[test]
        fn get() {
            let mut rodeo = Rodeo::default();
            let key = rodeo.get_or_intern("A");

            let reader = rodeo.into_reader();
            assert_eq!(Some(key), reader.get("A"));

            assert!(reader.get("F").is_none());
        }

        #[test]
        fn resolve() {
            let mut rodeo = Rodeo::default();
            let key = rodeo.get_or_intern("A");

            let reader = rodeo.into_reader();
            assert_eq!("A", reader.resolve(&key));
        }

        #[test]
        #[should_panic]
        #[cfg(not(miri))]
        fn resolve_panics() {
            let reader = Rodeo::default().into_reader();
            reader.resolve(&Spur::try_from_usize(100).unwrap());
        }

        #[test]
        fn try_resolve() {
            let mut rodeo = Rodeo::default();
            let key = rodeo.get_or_intern("A");

            let reader = rodeo.into_reader();
            assert_eq!(Some("A"), reader.try_resolve(&key));
            assert_eq!(
                None,
                reader.try_resolve(&Spur::try_from_usize(100).unwrap())
            );
        }

        #[test]
        fn resolve_unchecked() {
            let mut rodeo = Rodeo::default();
            let key = rodeo.get_or_intern("A");

            let reader = rodeo.into_reader();
            unsafe {
                assert_eq!("A", reader.resolve_unchecked(&key));
            }
        }

        #[test]
        fn len() {
            let mut rodeo = Rodeo::default();
            rodeo.get_or_intern("A");
            rodeo.get_or_intern("B");
            rodeo.get_or_intern("C");

            let reader = rodeo.into_reader();
            assert_eq!(reader.len(), 3);
        }

        #[test]
        fn empty() {
            let rodeo = Rodeo::default();
            let reader = rodeo.into_reader();

            assert!(reader.is_empty());
        }

        #[test]
        fn iter() {
            let mut rodeo = Rodeo::default();
            let a = rodeo.get_or_intern("a");
            let b = rodeo.get_or_intern("b");
            let c = rodeo.get_or_intern("c");

            let resolver = rodeo.into_reader();
            let mut iter = resolver.iter();

            assert_eq!(Some((a, "a")), iter.next());
            assert_eq!(Some((b, "b")), iter.next());
            assert_eq!(Some((c, "c")), iter.next());
            assert_eq!(None, iter.next());
        }

        #[test]
        fn strings() {
            let mut rodeo = Rodeo::default();
            rodeo.get_or_intern("a");
            rodeo.get_or_intern("b");
            rodeo.get_or_intern("c");

            let resolver = rodeo.into_reader();
            let mut iter = resolver.strings();

            assert_eq!(Some("a"), iter.next());
            assert_eq!(Some("b"), iter.next());
            assert_eq!(Some("c"), iter.next());
            assert_eq!(None, iter.next());
        }

        #[test]
        fn drops() {
            let rodeo = Rodeo::default();
            let _ = rodeo.into_reader();
        }

        #[test]
        fn into_resolver() {
            let mut rodeo = Rodeo::default();
            let key = rodeo.get_or_intern("A");

            let resolver = rodeo.into_reader().into_resolver();
            assert_eq!("A", resolver.resolve(&key));
        }

        #[test]
        #[cfg(not(any(feature = "no-std", feature = "ahasher")))]
        fn debug() {
            let reader = Rodeo::default().into_reader();
            println!("{:?}", reader);
        }
    }

    #[cfg(all(not(any(miri, feature = "no-std")), features = "multi-threaded"))]
    mod multi_threaded {
        use crate::{locks::Arc, multi_threaded::ThreadedRodeo};

        use std::thread;

        #[test]
        fn get() {
            let rodeo = ThreadedRodeo::default();
            let key = rodeo.intern("A");

            let reader = rodeo.into_reader();
            assert_eq!(Some(key), reader.get("A"));

            assert!(reader.get("F").is_none());
        }

        #[test]
        #[cfg(not(miri))]
        fn get_threaded() {
            let rodeo = ThreadedRodeo::default();
            let key = rodeo.intern("A");

            let reader = Arc::new(rodeo.into_reader());

            let moved = Arc::clone(&reader);
            thread::spawn(move || {
                assert_eq!(Some(key), moved.get("A"));
                assert!(moved.get("F").is_none());
            });

            assert_eq!(Some(key), reader.get("A"));
            assert!(reader.get("F").is_none());
        }

        #[test]
        fn resolve() {
            let rodeo = ThreadedRodeo::default();
            let key = rodeo.intern("A");

            let reader = rodeo.into_reader();
            assert_eq!("A", reader.resolve(&key));
        }

        #[test]
        #[cfg(not(miri))]
        fn resolve_threaded() {
            let rodeo = ThreadedRodeo::default();
            let key = rodeo.intern("A");

            let reader = Arc::new(rodeo.into_reader());

            let moved = Arc::clone(&reader);
            thread::spawn(move || {
                assert_eq!("A", moved.resolve(&key));
            });

            assert_eq!("A", reader.resolve(&key));
        }

        #[test]
        fn len() {
            let rodeo = ThreadedRodeo::default();
            rodeo.intern("A");
            rodeo.intern("B");
            rodeo.intern("C");

            let reader = rodeo.into_reader();
            assert_eq!(reader.len(), 3);
        }

        #[test]
        fn empty() {
            let rodeo = ThreadedRodeo::default();
            let reader = rodeo.into_reader();

            assert!(reader.is_empty());
        }

        #[test]
        fn clone() {
            let rodeo = ThreadedRodeo::default();
            let key = rodeo.intern("Test");

            let reader_rodeo = rodeo.into_reader();
            assert_eq!("Test", reader_rodeo.resolve(&key));

            let cloned = reader_rodeo.clone();
            assert_eq!("Test", cloned.resolve(&key));

            drop(reader_rodeo);

            assert_eq!("Test", cloned.resolve(&key));
        }

        #[test]
        fn iter() {
            let rodeo = ThreadedRodeo::default();
            let a = rodeo.get_or_intern("a");
            let b = rodeo.get_or_intern("b");
            let c = rodeo.get_or_intern("c");

            let resolver = rodeo.into_resolver();
            let mut iter = resolver.iter();

            assert_eq!(Some((a, "a")), iter.next());
            assert_eq!(Some((b, "b")), iter.next());
            assert_eq!(Some((c, "c")), iter.next());
            assert_eq!(None, iter.next());
        }

        #[test]
        fn strings() {
            let rodeo = ThreadedRodeo::default();
            rodeo.get_or_intern("a");
            rodeo.get_or_intern("b");
            rodeo.get_or_intern("c");

            let resolver = rodeo.into_resolver();
            let mut iter = resolver.strings();

            assert_eq!(Some("a"), iter.next());
            assert_eq!(Some("b"), iter.next());
            assert_eq!(Some("c"), iter.next());
            assert_eq!(None, iter.next());
        }

        #[test]
        fn drops() {
            let rodeo = ThreadedRodeo::default();
            let _ = rodeo.into_reader();
        }

        #[test]
        #[cfg(not(miri))]
        fn drop_threaded() {
            let rodeo = ThreadedRodeo::default();
            let reader = Arc::new(rodeo.into_reader());

            let moved = Arc::clone(&reader);
            thread::spawn(move || {
                let _ = moved;
            });
        }

        #[test]
        fn into_resolver() {
            let rodeo = ThreadedRodeo::default();
            let key = rodeo.intern("A");

            let resolver = rodeo.into_reader().into_resolver();
            assert_eq!("A", resolver.resolve(&key));
        }

        #[test]
        #[cfg(not(feature = "no-std"))]
        fn debug() {
            let reader = ThreadedRodeo::default().into_reader();
            println!("{:?}", reader);
        }
    }
}
