mod boxed;
mod rodeo;
mod rodeo_reader;
mod rodeo_resolver;
mod tests;
mod threaded_rodeo;

use crate::{LassoResult, Rodeo, RodeoReader, RodeoResolver, Spur};
#[cfg(feature = "no-std")]
use alloc::boxed::Box;
use sealed::Sealed;

/// A generic interface over any underlying interner, allowing storing and accessing
/// interned strings
pub trait Interner<K = Spur>: Reader<K> + Resolver<K> + Sealed {
    /// Get the key for a string, interning it if it does not yet exist
    ///
    /// # Panics
    ///
    /// Panics if the key's [`try_from_usize`](Key::try_from_usize) function fails. With the default
    /// keys, this means that you've interned more strings than it can handle. (For [`Spur`] this
    /// means that `u32::MAX - 1` unique strings were interned)
    ///
    fn get_or_intern(&mut self, val: &str) -> K;

    /// Get the key for a string, interning it if it does not yet exist
    fn try_get_or_intern(&mut self, val: &str) -> LassoResult<K>;

    /// Get the key for a static string, interning it if it does not yet exist
    ///
    /// This will not reallocate or copy the given string
    ///
    /// # Panics
    ///
    /// Panics if the key's [`try_from_usize`](Key::try_from_usize) function fails. With the default
    /// keys, this means that you've interned more strings than it can handle. (For [`Spur`] this
    /// means that `u32::MAX - 1` unique strings were interned)
    ///
    fn get_or_intern_static(&mut self, val: &'static str) -> K;

    /// Get the key for a static string, interning it if it does not yet exist
    ///
    /// This will not reallocate or copy the given string
    fn try_get_or_intern_static(&mut self, val: &'static str) -> LassoResult<K>;

    /// Consumes the current [`Interner`] and converts it into a [`RodeoReader`] to allow
    /// contention-free access of the interner from multiple threads
    #[must_use]
    fn into_reader(self) -> Box<dyn Reader<K>>
    where
        Self: 'static;

    /// An implementation detail to allow calling [`Interner::into_reader()`] on dynamically
    /// dispatched interners
    #[doc(hidden)]
    #[must_use]
    fn into_reader_boxed(self: Box<Self>) -> Box<dyn Reader<K>>
    where
        Self: 'static;
}

/// A generic interface that allows using any underlying interner for
/// both its reading and resolution capabilities, allowing both
/// `str -> key` and `key -> str` lookups
pub trait Reader<K = Spur>: Resolver<K> + Sealed {
    /// Get a key for the given string value if it exists
    fn get(&self, val: &str) -> Option<K>;

    /// Returns `true` if the current interner contains the given string
    fn contains(&self, val: &str) -> bool;

    /// Consumes the current [`Reader`] and makes it into a [`RodeoResolver`], allowing
    /// contention-free access from multiple threads with the lowest possible memory consumption
    #[must_use]
    fn into_resolver(self) -> RodeoResolver<K>;

    /// An implementation detail to allow calling [`Reader::into_resolver()`] on dynamically
    /// dispatched readers
    #[doc(hidden)]
    #[must_use]
    fn into_resolver_boxed(self: Box<Self>) -> RodeoResolver<K>;
}

/// A generic interface that allows using any underlying interner only
/// for its resolution capabilities, allowing only `key -> str` lookups
pub trait Resolver<K = Spur>: Sealed {
    /// Resolves the given key into a string
    ///
    /// # Panics
    ///
    /// Panics if the key is not contained in the current [`Resolver`]
    ///
    fn resolve<'a>(&'a self, key: &K) -> &'a str;

    /// Attempts to resolve the given key into a string, returning `None`
    /// if it cannot be found
    fn try_resolve<'a>(&'a self, key: &K) -> Option<&'a str>;

    /// Resolves a string by its key without preforming bounds checks
    ///
    /// # Safety
    ///
    /// The key must be valid for the current [`Resolver`]
    ///
    unsafe fn resolve_unchecked<'a>(&'a self, key: &K) -> &'a str;

    /// Returns `true` if the current interner contains the given key
    fn contains_key(&self, key: &K) -> bool;

    /// Gets the number of currently interned strings
    fn len(&self) -> usize;

    /// Returns `true` if there are no currently interned strings
    #[cfg_attr(feature = "inline-more", inline)]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

mod sealed {
    use super::*;

    /// A [sealed trait] to protect against downstream implementations
    ///
    /// [sealed trait]: https://rust-lang.github.io/api-guidelines/future-proofing.html#sealed-traits-protect-against-downstream-implementations-c-sealed
    pub trait Sealed {}

    impl<K, S> Sealed for Rodeo<K, S> {}

    impl<K> Sealed for RodeoResolver<K> {}

    impl<K, S> Sealed for RodeoReader<K, S> {}

    #[cfg(feature = "multi-threaded")]
    impl<K, S> Sealed for crate::ThreadedRodeo<K, S> {}

    impl<I: ?Sized> Sealed for Box<I> {}
}
