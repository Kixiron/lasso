use crate::{
    arena::Arena,
    key::{Key, Spur},
    util::{Iter, Strings},
};
use core::marker::PhantomData;

compile! {
    if #[feature = "no-std"] {
        use alloc::vec::Vec;
    }
}

/// A read-only view of a [`Rodeo`] or [`ThreadedRodeo`] that allows contention-free access to interned strings
/// with only key to string resolution
///
/// The key type is the same as the `Rodeo` or `ThreadedRodeo` that created it
///
/// [`Rodeo`]: crate::Rodeo
/// [`ThreadedRodeo`]: crate::ThreadedRodeo
#[derive(Debug)]
pub struct RodeoResolver<K = Spur> {
    /// Vector of strings mapped to key indexes that allows key to string resolution
    pub(crate) strings: Vec<&'static str>,
    /// The arena that contains all the strings
    ///
    /// This is not touched, but *must* be kept since every string in `self.strings`
    /// points to it
    __arena: Arena,
    /// The type of the key
    __key: PhantomData<K>,
}

impl<K> RodeoResolver<K> {
    /// Creates a new RodeoResolver
    ///
    /// # Safety
    ///
    /// The references inside of `strings` must be absolutely unique, meaning
    /// that no other references to those strings exist
    ///
    pub(crate) unsafe fn new(strings: Vec<&'static str>, arena: Arena) -> Self {
        Self {
            strings,
            __arena: arena,
            __key: PhantomData,
        }
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

    /// Resolves a string by its key without bounds checking
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
    /// let rodeo = rodeo.into_resolver();
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
    /// let rodeo = rodeo.into_resolver();
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
        Iter::from_resolver(self)
    }

    /// Returns an iterator over the interned strings
    #[inline]
    pub fn strings(&self) -> Strings<'_, K> {
        Strings::from_resolver(self)
    }
}

unsafe impl<K: Send> Send for RodeoResolver<K> {}
unsafe impl<K: Sync> Sync for RodeoResolver<K> {}

#[cfg(test)]
mod tests {
    mod single_threaded {
        use crate::{single_threaded::Rodeo, Key, Spur};

        #[test]
        fn resolve() {
            let mut rodeo = Rodeo::default();
            let key = rodeo.get_or_intern("A");

            let resolver = rodeo.into_resolver();
            assert_eq!("A", resolver.resolve(&key));
        }

        #[test]
        #[should_panic]
        #[cfg(not(miri))]
        fn resolve_out_of_bounds() {
            let resolver = Rodeo::default().into_resolver();
            resolver.resolve(&Spur::try_from_usize(10).unwrap());
        }

        #[test]
        fn try_resolve() {
            let mut rodeo = Rodeo::default();
            let key = rodeo.get_or_intern("A");

            let resolver = rodeo.into_resolver();
            assert_eq!(Some("A"), resolver.try_resolve(&key));
            assert_eq!(
                None,
                resolver.try_resolve(&Spur::try_from_usize(10).unwrap())
            );
        }

        #[test]
        fn resolve_unchecked() {
            let mut rodeo = Rodeo::default();
            let a = rodeo.get_or_intern("A");

            let resolver = rodeo.into_resolver();
            unsafe {
                assert_eq!("A", resolver.resolve_unchecked(&a));
            }
        }

        #[test]
        fn len() {
            let mut rodeo = Rodeo::default();
            rodeo.get_or_intern("A");
            rodeo.get_or_intern("B");
            rodeo.get_or_intern("C");

            let resolver = rodeo.into_resolver();
            assert_eq!(resolver.len(), 3);
        }

        #[test]
        fn empty() {
            let rodeo = Rodeo::default();
            let read_only = rodeo.into_resolver();

            assert!(read_only.is_empty());
        }

        #[test]
        fn drops() {
            let rodeo = Rodeo::default();
            let _ = rodeo.into_resolver();
        }

        #[test]
        fn iter() {
            let mut rodeo = Rodeo::default();
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
            let mut rodeo = Rodeo::default();
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
        #[cfg(not(feature = "no-std"))]
        fn debug() {
            let resolver = Rodeo::default().into_resolver();
            println!("{:?}", resolver);
        }
    }

    #[cfg(all(not(any(miri, feature = "no-std")), features = "multi-threaded"))]
    mod multi_threaded {
        use crate::{locks::Arc, multi_threaded::ThreadedRodeo};
        use std::thread;

        #[test]
        fn resolve() {
            let rodeo = ThreadedRodeo::default();
            let key = rodeo.intern("A");

            let resolver = rodeo.into_resolver();
            assert_eq!("A", resolver.resolve(&key));
        }

        #[test]
        fn try_resolve() {
            let mut rodeo = ThreadedRodeo::default();
            let key = rodeo.intern("A");

            let resolver = rodeo.into_resolver();
            assert_eq!(Some("A"), resolver.try_resolve(&key));
            assert_eq!(
                None,
                resolver.try_resolve(&Spur::try_from_usize(10).unwrap())
            );
        }

        #[test]
        #[cfg(not(miri))]
        fn try_resolve_threaded() {
            let rodeo = ThreadedRodeo::default();
            let key = rodeo.intern("A");

            let resolver = Arc::new(rodeo.into_resolver());

            let moved = Arc::clone(&resolver);
            thread::spawn(move || {
                assert_eq!(Some("A"), resolver.try_resolve(&key));
                assert_eq!(
                    None,
                    resolver.try_resolve(&Spur::try_from_usize(10).unwrap())
                );
            });

            assert_eq!(Some("A"), resolver.try_resolve(&key));
            assert_eq!(
                None,
                resolver.try_resolve(&Spur::try_from_usize(10).unwrap())
            );
        }

        #[test]
        fn resolve_unchecked() {
            let mut rodeo = ThreadedRodeo::default();
            let a = rodeo.get_or_intern("A");

            let resolver = rodeo.into_resolver();
            unsafe {
                assert_eq!("A", resolver.resolve_unchecked(&a));
            }
        }

        #[test]
        #[cfg(not(miri))]
        fn resolve_unchecked_threaded() {
            let rodeo = ThreadedRodeo::default();
            let key = rodeo.intern("A");

            let resolver = Arc::new(rodeo.into_resolver());

            let moved = Arc::clone(&resolver);
            thread::spawn(move || unsafe {
                assert_eq!("A", moved.resolve_unchecked(&a));
            });

            unsafe {
                assert_eq!("A", resolver.resolve_unchecked(&a));
            }
        }

        #[test]
        #[cfg(not(miri))]
        fn resolve_threaded() {
            let rodeo = ThreadedRodeo::default();
            let key = rodeo.intern("A");

            let resolver = Arc::new(rodeo.into_resolver());

            let moved = Arc::clone(&resolver);
            thread::spawn(move || {
                assert_eq!("A", moved.resolve(&key));
            });

            assert_eq!("A", resolver.resolve(&key));
        }

        #[test]
        fn len() {
            let rodeo = ThreadedRodeo::default();
            rodeo.intern("A");
            rodeo.intern("B");
            rodeo.intern("C");

            let resolver = rodeo.into_resolver();
            assert_eq!(resolver.len(), 3);
        }

        #[test]
        fn empty() {
            let rodeo = ThreadedRodeo::default();
            let read_only = rodeo.into_resolver();

            assert!(read_only.is_empty());
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
        fn clone() {
            let rodeo = ThreadedRodeo::default();
            let key = rodeo.intern("Test");

            let resolver_rodeo = rodeo.into_resolver();
            assert_eq!("Test", resolver_rodeo.resolve(&key));

            let cloned = resolver_rodeo.clone();
            assert_eq!("Test", cloned.resolve(&key));

            drop(resolver_rodeo);

            assert_eq!("Test", cloned.resolve(&key));
        }

        #[test]
        fn drops() {
            let rodeo = ThreadedRodeo::default();
            let _ = rodeo.into_resolver();
        }

        #[test]
        #[cfg(not(miri))]
        fn drop_threaded() {
            let rodeo = ThreadedRodeo::default();
            let resolver = Arc::new(rodeo.into_resolver());

            let moved = Arc::clone(&resolver);
            thread::spawn(move || {
                let _ = moved;
            });
        }

        #[test]
        #[cfg(not(feature = "no-std"))]
        fn debug() {
            let resolver = ThreadedRodeo::default().into_resolver();
            println!("{:?}", resolver);
        }
    }
}
