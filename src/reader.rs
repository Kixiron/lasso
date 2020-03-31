use super::RodeoResolver;
use crate::{
    hasher::{HashMap, RandomState},
    key::{Key, Spur},
    util::{Iter, Strings},
};

use core::{hash::BuildHasher, mem};

compile! {
    if #[feature = "no_std"] {
        use alloc::{vec::Vec, string::ToString, boxed::Box};
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
#[derive(Debug)]
pub struct RodeoReader<K: Key = Spur, S: BuildHasher + Clone = RandomState> {
    map: HashMap<&'static str, K, S>,
    pub(crate) strings: Vec<&'static str>,
}

impl<K: Key, S: BuildHasher + Clone> RodeoReader<K, S> {
    /// Creates a new RodeoReader
    ///
    /// # Safety
    ///
    /// The references inside of `strings` must be absolutely unique, meaning
    /// that no other references to those strings exist
    ///
    pub(crate) unsafe fn new(map: HashMap<&'static str, K, S>, strings: Vec<&'static str>) -> Self {
        Self { map, strings }
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
    {
        self.map.get(val.as_ref()).map(|k| *k)
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
    pub fn iter(&self) -> Iter<K> {
        Iter::from_reader(self)
    }

    /// Returns an iterator over the interned strings
    #[inline]
    pub fn strings(&self) -> Strings<K> {
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
    pub fn into_resolver(mut self) -> RodeoResolver<K> {
        let strings = mem::take(&mut self.strings);

        // Safety: The current reader no longer contains references to the strings
        // in the vec given to RodeoResolver
        unsafe { RodeoResolver::new(strings) }
    }
}

impl<K, S> Clone for RodeoReader<K, S>
where
    K: Key,
    S: BuildHasher + Clone,
{
    fn clone(&self) -> Self {
        // Safety: The strings of the current Reader **cannot** be used in the new Reader

        // Create the new map/vec that will fill the new Reader, pre-allocating their capacity
        let mut map =
            HashMap::with_capacity_and_hasher(self.strings.len(), self.map.hasher().clone());
        let mut strings = Vec::with_capacity(self.strings.len());

        // For each string in the to-be-cloned Reader, take ownership of each string by calling to_string,
        // therefore cloning it onto the heap, calling into_boxed_str and leaking that
        for (i, string) in self.strings.iter().enumerate() {
            // Clone the static string from self.strings onto the heap, box and leak it
            let new: &'static str = Box::leak(string.to_string().into_boxed_str());

            // Store the new string, which we have ownership of, in the new map and vec
            strings.push(new);
            // The indices of the vector correspond with the keys
            map.insert(new, K::try_from_usize(i).unwrap_or_else(|| unreachable!()));
        }

        Self { map, strings }
    }
}

/// Deallocate the leaked strings interned by RodeoReader
impl<K: Key, S: BuildHasher + Clone> Drop for RodeoReader<K, S> {
    fn drop(&mut self) {
        // Clear the map to remove all other references to the strings in self.strings
        self.map.clear();

        // Drain self.strings while deallocating the strings it holds
        for string in self.strings.drain(..) {
            // Safety: There must not be any other references to the strings being re-boxed, so the
            // map containing all other references is first drained, leaving the sole reference to
            // the strings vector, which allows the safe dropping of the string. This also relies on the
            // implemented functions for RodeoReader not giving out any references to the strings it holds
            // that live beyond itself. It also relies on the Clone implementation of RodeoReader to clone and
            // take ownership of all the interned strings as to not have a double free when one is dropped
            unsafe {
                let _ = Box::from_raw(string as *const str as *mut str);
            }
        }
    }
}

unsafe impl<K: Key + Sync, S: BuildHasher + Clone + Sync> Sync for RodeoReader<K, S> {}
unsafe impl<K: Key + Send, S: BuildHasher + Clone + Send> Send for RodeoReader<K, S> {}

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
        fn clone() {
            let mut rodeo = Rodeo::default();
            let key = rodeo.get_or_intern("Test");

            let reader_rodeo = rodeo.into_reader();
            assert_eq!("Test", reader_rodeo.resolve(&key));

            let cloned = reader_rodeo.clone();
            assert_eq!("Test", cloned.resolve(&key));

            drop(reader_rodeo);

            assert_eq!("Test", cloned.resolve(&key));
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
        #[cfg(not(feature = "no_std"))]
        fn debug() {
            let reader = Rodeo::default().into_reader();
            println!("{:?}", reader);
        }
    }

    #[cfg(all(not(any(miri, feature = "no_std")), features = "multi-threaded"))]
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
        #[cfg(not(feature = "no_std"))]
        fn debug() {
            let reader = ThreadedRodeo::default().into_reader();
            println!("{:?}", reader);
        }
    }
}
