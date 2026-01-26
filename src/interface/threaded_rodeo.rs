//! Implementations of [`Interner`], [`Reader`] and [`Resolver`] for [`ThreadedRodeo`]
#![cfg(feature = "multi-threaded")]

use crate::rodeo::Internable;
use crate::*;
#[cfg(feature = "no-std")]
use alloc::boxed::Box;
use core::hash::{BuildHasher, Hash};

impl<T, K, S> Interner<T, K> for ThreadedRodeo<T, K, S>
where
    T: Internable,
    K: Key + Hash,
    S: BuildHasher + Clone,
{
    #[cfg_attr(feature = "inline-more", inline)]
    fn get_or_intern(&mut self, val: &T::Ref) -> K {
        (*self).get_or_intern(val)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn try_get_or_intern(&mut self, val: &T::Ref) -> LassoResult<K> {
        (*self).try_get_or_intern(val)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn get_or_intern_static(&mut self, val: &'static T::Ref) -> K {
        (*self).get_or_intern_static(val)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn try_get_or_intern_static(&mut self, val: &'static T::Ref) -> LassoResult<K> {
        (*self).try_get_or_intern_static(val)
    }
}

impl<T, K, S> IntoReaderAndResolver<T, K> for ThreadedRodeo<T, K, S>
where
    T: Internable,
    K: Key + Hash,
    S: BuildHasher + Clone,
{
}

impl<T, K, S> IntoReader<T, K> for ThreadedRodeo<T, K, S>
where
    T: Internable,
    K: Key + Hash,
    S: BuildHasher + Clone,
{
    type Reader = RodeoReader<T, K, S>;

    #[cfg_attr(feature = "inline-more", inline)]
    fn into_reader(self) -> Self::Reader
    where
        Self: 'static,
    {
        self.into_reader()
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn into_reader_boxed(self: Box<Self>) -> Self::Reader
    where
        Self: 'static,
    {
        ThreadedRodeo::into_reader(*self)
    }
}

impl<T, K, S> Reader<T, K> for ThreadedRodeo<T, K, S>
where
    T: Internable,
    K: Key + Hash,
    S: BuildHasher + Clone,
{
    #[cfg_attr(feature = "inline-more", inline)]
    fn get(&self, val: &T::Ref) -> Option<K> {
        self.get(val)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn contains(&self, val: &T::Ref) -> bool {
        self.contains(val)
    }
}

impl<T, K, S> IntoResolver<T, K> for ThreadedRodeo<T, K, S>
where
    T: Internable,
    K: Key + Hash,
    S: BuildHasher + Clone,
{
    type Resolver = RodeoResolver<T, K>;

    #[cfg_attr(feature = "inline-more", inline)]
    fn into_resolver(self) -> Self::Resolver
    where
        Self: 'static,
    {
        self.into_resolver()
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn into_resolver_boxed(self: Box<Self>) -> Self::Resolver
    where
        Self: 'static,
    {
        ThreadedRodeo::into_resolver(*self)
    }
}

impl<T, K, S> Resolver<T, K> for ThreadedRodeo<T, K, S>
where
    T: Internable,
    K: Key + Hash,
    S: BuildHasher + Clone,
{
    #[cfg_attr(feature = "inline-more", inline)]
    fn resolve<'a>(&'a self, key: &K) -> &'a T::Ref {
        self.resolve(key)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn try_resolve<'a>(&'a self, key: &K) -> Option<&'a T::Ref> {
        self.try_resolve(key)
    }

    /// [`ThreadedRodeo`] does not actually have a `resolve_unchecked()` method,
    /// so this just forwards to the normal [`ThreadedRodeo::resolve()`] method
    #[cfg_attr(feature = "inline-more", inline)]
    unsafe fn resolve_unchecked<'a>(&'a self, key: &K) -> &'a T::Ref {
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
