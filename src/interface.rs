#[cfg(feature = "multi-threaded")]
use crate::ThreadedRodeo;
use crate::{Key, Rodeo, RodeoReader, RodeoResolver};
use core::hash::{BuildHasher, Hash};
use sealed::Sealed;

/// A generic interface that allows using any underlying interner for
/// both its reading and resolution capabilities, allowing both
/// `str -> key` and `key -> str` lookups
pub trait Reader<K>: Resolver<K> + Sealed {
    /// Get a key for the given string value if it exists
    fn get(&self, val: &str) -> Option<K>;

    /// Returns `true` if the current interner contains the given string
    fn contains(&self, val: &str) -> bool;
}

/// A generic interface that allows using any underlying interner only
/// for its resolution capabilities, allowing only `key -> str` lookups
pub trait Resolver<K>: Sealed {
    /// Resolves the given key into a string, panicking if it cannot be found
    fn resolve<'a>(&'a self, key: &K) -> &'a str;

    /// Attempts to resolve the given key into a string, returning `None`
    /// if it cannot be found
    fn try_resolve<'a>(&'a self, key: &K) -> Option<&'a str>;

    /// Returns `true` if the current interner contains the given key
    fn contains_key(&self, key: &K) -> bool;
}

impl<K, S> Reader<K> for Rodeo<K, S>
where
    K: Key,
    S: BuildHasher,
{
    #[cfg_attr(feature = "inline-more", inline)]
    fn get(&self, val: &str) -> Option<K> {
        self.get(val)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn contains(&self, val: &str) -> bool {
        self.contains(val)
    }
}

impl<K, S> Resolver<K> for Rodeo<K, S>
where
    K: Key,
    S: BuildHasher,
{
    #[cfg_attr(feature = "inline-more", inline)]
    fn resolve<'a>(&'a self, key: &K) -> &'a str {
        self.resolve(key)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn try_resolve<'a>(&'a self, key: &K) -> Option<&'a str> {
        self.try_resolve(key)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn contains_key(&self, key: &K) -> bool {
        self.contains_key(key)
    }
}

impl<K, S> Reader<K> for RodeoReader<K, S>
where
    K: Key,
    S: BuildHasher,
{
    #[cfg_attr(feature = "inline-more", inline)]
    fn get(&self, val: &str) -> Option<K> {
        self.get(val)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn contains(&self, val: &str) -> bool {
        self.contains(val)
    }
}

impl<K, S> Resolver<K> for RodeoReader<K, S>
where
    K: Key,
{
    #[cfg_attr(feature = "inline-more", inline)]
    fn resolve<'a>(&'a self, key: &K) -> &'a str {
        self.resolve(key)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn try_resolve<'a>(&'a self, key: &K) -> Option<&'a str> {
        self.try_resolve(key)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn contains_key(&self, key: &K) -> bool {
        self.contains_key(key)
    }
}

impl<K> Resolver<K> for RodeoResolver<K>
where
    K: Key,
{
    #[cfg_attr(feature = "inline-more", inline)]
    fn resolve<'a>(&'a self, key: &K) -> &'a str {
        self.resolve(key)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn try_resolve<'a>(&'a self, key: &K) -> Option<&'a str> {
        self.try_resolve(key)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn contains_key(&self, key: &K) -> bool {
        self.contains_key(key)
    }
}

#[cfg(feature = "multi-threaded")]
impl<K, S> Reader<K> for ThreadedRodeo<K, S>
where
    K: Key + Hash,
    S: BuildHasher + Clone,
{
    #[cfg_attr(feature = "inline-more", inline)]
    fn get(&self, val: &str) -> Option<K> {
        self.get(val)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn contains(&self, val: &str) -> bool {
        self.contains(val)
    }
}

#[cfg(feature = "multi-threaded")]
impl<K, S> Resolver<K> for ThreadedRodeo<K, S>
where
    K: Key + Hash,
    S: BuildHasher + Clone,
{
    #[cfg_attr(feature = "inline-more", inline)]
    fn resolve<'a>(&'a self, key: &K) -> &'a str {
        self.resolve(key)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn try_resolve<'a>(&'a self, key: &K) -> Option<&'a str> {
        self.try_resolve(key)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn contains_key(&self, key: &K) -> bool {
        self.contains_key(key)
    }
}

mod sealed {
    use super::*;

    pub trait Sealed {}

    impl<K, S> Sealed for Rodeo<K, S> {}
    impl<K> Sealed for RodeoResolver<K> {}
    impl<K, S> Sealed for RodeoReader<K, S> {}
    impl<K, S> Sealed for ThreadedRodeo<K, S> {}
}

// TODO: Figure out an interface that suits both `Rodeo`'s required mutability
//       and `ThreadedRodeo`'s interior mutability
//
// pub trait Interner<K>: Reader<K> + Resolver<K> {
//     fn get_or_intern(&mut self, val: &str) -> K;
//
//     fn try_get_or_intern(&mut self, val: &str) -> LassoResult<K>;
//
//     fn get_or_intern_static(&mut self, val: &'static str) -> K;
//
//     fn try_get_or_intern_static(&mut self, val: &'static str) -> LassoResult<K>;
// }
