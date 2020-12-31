//! Implementations of [`Interner`], [`Reader`] and [`Resolver`] for [`Rodeo`]

use crate::{Interner, Key, LassoResult, Reader, Resolver, Rodeo, RodeoResolver};
#[cfg(feature = "no-std")]
use alloc::boxed::Box;
use core::hash::BuildHasher;

impl<K, S> Interner<K> for Rodeo<K, S>
where
    K: Key,
    S: BuildHasher,
{
    #[cfg_attr(feature = "inline-more", inline)]
    fn get_or_intern(&mut self, val: &str) -> K {
        self.get_or_intern(val)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn try_get_or_intern(&mut self, val: &str) -> LassoResult<K> {
        self.try_get_or_intern(val)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn get_or_intern_static(&mut self, val: &'static str) -> K {
        self.get_or_intern_static(val)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn try_get_or_intern_static(&mut self, val: &'static str) -> LassoResult<K> {
        self.try_get_or_intern_static(val)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    #[must_use]
    fn into_reader(self) -> Box<dyn Reader<K>>
    where
        Self: 'static,
    {
        Box::new(self).into_reader_boxed()
    }

    #[cfg_attr(feature = "inline-more", inline)]
    #[must_use]
    fn into_reader_boxed(self: Box<Self>) -> Box<dyn Reader<K>>
    where
        Self: 'static,
    {
        Box::new(Rodeo::into_reader(*self))
    }
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

    #[cfg_attr(feature = "inline-more", inline)]
    #[must_use]
    fn into_resolver(self) -> RodeoResolver<K> {
        self.into_resolver()
    }

    #[cfg_attr(feature = "inline-more", inline)]
    #[must_use]
    fn into_resolver_boxed(self: Box<Self>) -> RodeoResolver<K> {
        (*self).into_resolver()
    }
}

impl<K, S> Resolver<K> for Rodeo<K, S>
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
