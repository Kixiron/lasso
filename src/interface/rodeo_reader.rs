//! Implementations of [`Reader`] and [`Resolver`] for [`RodeoReader`]

use crate::{Key, Reader, Resolver, RodeoReader, RodeoResolver};
use core::hash::BuildHasher;

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

    #[cfg_attr(feature = "inline-more", inline)]
    #[must_use]
    fn into_resolver(self) -> RodeoResolver<K> {
        self.into_resolver()
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
    unsafe fn resolve_unchecked<'a>(&'a self, key: &K) -> &'a str {
        self.resolve_unchecked(key)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn contains_key(&self, key: &K) -> bool {
        self.contains_key(key)
    }
}
