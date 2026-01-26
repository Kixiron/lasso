//! Implementations of [`Interner`], [`Reader`] and [`Resolver`] for [`Rodeo`]

use crate::{rodeo::Internable, *};
#[cfg(feature = "no-std")]
use alloc::boxed::Box;
use core::hash::BuildHasher;
use interface::IntoReaderAndResolver;

impl<T: Internable, K, S> Interner<T, K> for Rodeo<T, K, S>
where
    K: Key,
    S: BuildHasher,
{
    #[cfg_attr(feature = "inline-more", inline)]
    fn get_or_intern(&mut self, val: &T::Ref) -> K {
        self.get_or_intern(val)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn try_get_or_intern(&mut self, val: &T::Ref) -> LassoResult<K> {
        self.try_get_or_intern(val)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn get_or_intern_static(&mut self, val: &'static T::Ref) -> K {
        self.get_or_intern_static(val)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn try_get_or_intern_static(&mut self, val: &'static T::Ref) -> LassoResult<K> {
        self.try_get_or_intern_static(val)
    }
}

impl<T: Internable, K, S> IntoReaderAndResolver<T, K> for Rodeo<T, K, S>
where
    K: Key,
    S: BuildHasher,
{
}

impl<T: Internable, K, S> IntoReader<T, K> for Rodeo<T, K, S>
where
    K: Key,
    S: BuildHasher,
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
        Rodeo::into_reader(*self)
    }
}

impl<T: Internable, K, S> Reader<T, K> for Rodeo<T, K, S>
where
    K: Key,
    S: BuildHasher,
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

impl<T: Internable, K, S> IntoResolver<T, K> for Rodeo<T, K, S>
where
    K: Key,
    S: BuildHasher,
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
        Rodeo::into_resolver(*self)
    }
}

impl<T: Internable, K, S> Resolver<T, K> for Rodeo<T, K, S>
where
    K: Key,
{
    #[cfg_attr(feature = "inline-more", inline)]
    fn resolve<'a>(&'a self, key: &K) -> &'a T::Ref {
        self.resolve(key)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn try_resolve<'a>(&'a self, key: &K) -> Option<&'a T::Ref> {
        self.try_resolve(key)
    }

    #[cfg_attr(feature = "inline-more", inline)]
    unsafe fn resolve_unchecked<'a>(&'a self, key: &K) -> &'a T::Ref {
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
