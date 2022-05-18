//! Implementations of [`Interner`], [`Reader`] and [`Resolver`] for [`Rodeo`]

use crate::*;
#[cfg(feature = "no-std")]
use alloc::boxed::Box;
use core::hash::BuildHasher;
use interface::IntoReaderAndResolver;

impl<K, V, S> Interner<K, V> for Rodeo<K, V, S>
where
    K: Key,
    V: ?Sized + Internable,
    S: BuildHasher,
{
    #[cfg_attr(feature = "inline-more", inline)]
    fn get_or_intern(&mut self, val: &V) -> K {
        self.get_or_intern(val)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn try_get_or_intern(&mut self, val: &V) -> LassoResult<K> {
        self.try_get_or_intern(val)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn get_or_intern_static(&mut self, val: &'static V) -> K {
        self.get_or_intern_static(val)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn try_get_or_intern_static(&mut self, val: &'static V) -> LassoResult<K> {
        self.try_get_or_intern_static(val)
    }
}

impl<K, V, S> IntoReaderAndResolver<K, V> for Rodeo<K, V, S>
where
    K: Key,
    V: ?Sized + Internable,
    S: BuildHasher,
{
}

impl<K, V, S> IntoReader<K, V> for Rodeo<K, V, S>
where
    K: Key,
    V: ?Sized + Internable,
    S: BuildHasher,
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
        Rodeo::into_reader(*self)
    }
}

impl<K, V, S> Reader<K, V> for Rodeo<K, V, S>
where
    K: Key,
    V: ?Sized + Internable,
    S: BuildHasher,
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

impl<K, V, S> IntoResolver<K, V> for Rodeo<K, V, S>
where
    K: Key,
    V: ?Sized + Internable,
    S: BuildHasher,
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
        Rodeo::into_resolver(*self)
    }
}

impl<K, V, S> Resolver<K, V> for Rodeo<K, V, S>
where
    K: Key,
    V: ?Sized + Internable,
{
    #[cfg_attr(feature = "inline-more", inline)]
    fn resolve<'a>(&'a self, key: &K) -> &'a V {
        self.resolve(key)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn try_resolve<'a>(&'a self, key: &K) -> Option<&'a V> {
        self.try_resolve(key)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    unsafe fn resolve_unchecked<'a>(&'a self, key: &K) -> &'a V {
        unsafe { self.resolve_unchecked(key) }
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
