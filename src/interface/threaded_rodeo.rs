//! Implementations of [`Interner`], [`Reader`] and [`Resolver`] for [`ThreadedRodeo`]
#![cfg(feature = "multi-threaded")]

use crate::*;
#[cfg(feature = "no-std")]
use alloc::boxed::Box;
use core::hash::{BuildHasher, Hash};

impl<K, V, S> Interner<K, V> for ThreadedRodeo<K, V, S>
where
    K: Key + Hash,
    V: ?Sized + Internable,
    S: BuildHasher + Clone,
{
    #[cfg_attr(feature = "inline-more", inline)]
    fn get_or_intern(&mut self, val: &V) -> K {
        (&*self).get_or_intern(val)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn try_get_or_intern(&mut self, val: &V) -> LassoResult<K> {
        (&*self).try_get_or_intern(val)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn get_or_intern_static(&mut self, val: &'static V) -> K {
        (&*self).get_or_intern_static(val)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn try_get_or_intern_static(&mut self, val: &'static V) -> LassoResult<K> {
        (&*self).try_get_or_intern_static(val)
    }
}

impl<K, V, S> IntoReaderAndResolver<K, V> for ThreadedRodeo<K, V, S>
where
    K: Key + Hash,
    V: ?Sized + Internable,
    S: BuildHasher + Clone,
{
}

impl<K, V, S> IntoReader<K, V> for ThreadedRodeo<K, V, S>
where
    K: Key + Hash,
    V: ?Sized + Internable,
    S: BuildHasher + Clone,
{
    type Reader = RodeoReader<K, V, S>;

    #[cfg_attr(feature = "inline-more", inline)]
    #[must_use]
    fn into_reader(self) -> Self::Reader
    where
        Self: 'static,
    {
        self.into_reader()
    }

    #[cfg_attr(feature = "inline-more", inline)]
    #[must_use]
    fn into_reader_boxed(self: Box<Self>) -> Self::Reader
    where
        Self: 'static,
    {
        ThreadedRodeo::into_reader(*self)
    }
}

impl<K, V, S> Reader<K, V> for ThreadedRodeo<K, V, S>
where
    K: Key + Hash,
    V: ?Sized + Internable,
    S: BuildHasher + Clone,
{
    #[cfg_attr(feature = "inline-more", inline)]
    fn get(&self, val: &V) -> Option<K> {
        self.get(val)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn contains(&self, val: &V) -> bool {
        self.contains(val)
    }
}

impl<K, V, S> IntoResolver<K, V> for ThreadedRodeo<K, V, S>
where
    K: Key + Hash,
    V: ?Sized + Internable,
    S: BuildHasher + Clone,
{
    type Resolver = RodeoResolver<K, V>;

    #[cfg_attr(feature = "inline-more", inline)]
    #[must_use]
    fn into_resolver(self) -> Self::Resolver
    where
        Self: 'static,
    {
        self.into_resolver()
    }

    #[cfg_attr(feature = "inline-more", inline)]
    #[must_use]
    fn into_resolver_boxed(self: Box<Self>) -> Self::Resolver
    where
        Self: 'static,
    {
        ThreadedRodeo::into_resolver(*self)
    }
}

impl<K, V, S> Resolver<K, V> for ThreadedRodeo<K, V, S>
where
    K: Key + Hash,
    V: ?Sized + Internable,
    S: BuildHasher + Clone,
{
    #[cfg_attr(feature = "inline-more", inline)]
    fn resolve<'a>(&'a self, key: &K) -> &'a V {
        self.resolve(key)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn try_resolve<'a>(&'a self, key: &K) -> Option<&'a V> {
        self.try_resolve(key)
    }

    /// [`ThreadedRodeo`] does not actually have a `resolve_unchecked()` method,
    /// so this just forwards to the normal [`ThreadedRodeo::resolve()`] method
    #[cfg_attr(feature = "inline-more", inline)]
    unsafe fn resolve_unchecked<'a>(&'a self, key: &K) -> &'a V {
        self.resolve(key)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn contains_key(&self, key: &K) -> bool {
        self.contains_key(key)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn len(&self) -> usize {
        self.len()
    }
}
